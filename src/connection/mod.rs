// Copyright 2020 - 2021 Alex Dukhno
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    connection::network::*, BackendMessage, CommandMessage, ConnId, ConnSecretKey, MessageDecoder, MessageDecoderStatus,
};
use async_mutex::Mutex as AsyncMutex;
use futures_lite::future::block_on;
use rand::Rng;
use std::{
    collections::{HashMap, VecDeque},
    io,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[cfg(feature = "async_net")]
mod async_native_tls;
pub mod listener;
pub mod network;

/// An error returned from creating an acceptor.
#[derive(Debug)]
pub enum AcceptError {
    /// NativeTls error.
    NativeTls(native_tls::Error),
    /// Io error.
    Io(std::io::Error),
}

impl From<native_tls::Error> for AcceptError {
    fn from(error: native_tls::Error) -> AcceptError {
        AcceptError::NativeTls(error)
    }
}

impl From<std::io::Error> for AcceptError {
    fn from(error: std::io::Error) -> AcceptError {
        AcceptError::Io(error)
    }
}

type Props = Vec<(String, String)>;

pub struct Connection {
    id: ConnId,
    #[allow(dead_code)]
    client_props: Props,
    #[allow(dead_code)]
    address: SocketAddr,
    channel: Arc<AsyncMutex<Channel>>,
    supervisor: ConnSupervisor,
    sender: Arc<ResponseSender>,
}

impl Connection {
    pub fn new(
        id: ConnId,
        client_props: Props,
        address: SocketAddr,
        channel: Arc<AsyncMutex<Channel>>,
        supervisor: ConnSupervisor,
    ) -> Connection {
        let sender = Arc::new(ResponseSender::new(channel.clone()));
        Connection {
            id,
            client_props,
            address,
            channel,
            supervisor,
            sender,
        }
    }

    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    pub fn sender(&self) -> Arc<ResponseSender> {
        self.sender.clone()
    }

    async fn read_frontend_message(&mut self) -> io::Result<Result<CommandMessage, ()>> {
        let mut current: Option<Vec<u8>> = None;
        let mut message_decoder = MessageDecoder::default();
        loop {
            match message_decoder.next_stage(current.take().as_deref()) {
                Ok(MessageDecoderStatus::Requesting(len)) => {
                    let mut buffer = vec![b'0'; len];
                    self.channel.lock().await.read_exact(&mut buffer).await?;
                    current = Some(buffer);
                }
                Ok(MessageDecoderStatus::Done(message)) => return Ok(Ok(message)),
                Err(_error) => {
                    return Ok(Err(()));
                }
            }
        }
    }

    pub async fn receive(&mut self) -> io::Result<Result<CommandMessage, ()>> {
        let message = match self.read_frontend_message().await {
            Ok(Ok(message)) => message,
            Ok(Err(_err)) => return Ok(Err(())),
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                // Client disconnected the socket immediately without sending a
                // Terminate message. Considers it as a client Terminate to save
                // resource and exit smoothly.
                CommandMessage::Terminate
            }
            Err(err) => return Err(err),
        };
        Ok(Ok(message))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.supervisor.free(self.id);
    }
}

/// Client request accepted from a client
pub enum ClientRequest {
    /// Connection to perform queries
    Connect(Connection),
    /// Connection to cancel queries of another client
    QueryCancellation(ConnId),
}

/// Responsible for sending messages back to client
pub struct ResponseSender {
    channel: Arc<AsyncMutex<Channel>>,
}

impl ResponseSender {
    pub(crate) fn new(channel: Arc<AsyncMutex<Channel>>) -> ResponseSender {
        ResponseSender { channel }
    }
}

impl Sender for ResponseSender {
    fn flush(&self) -> io::Result<()> {
        block_on(async {
            self.channel.lock().await.flush().await.expect("OK");
        });

        Ok(())
    }

    fn send(&self, query_result: Result<BackendMessage, BackendMessage>) -> io::Result<()> {
        block_on(async {
            let message: BackendMessage = match query_result {
                Ok(event) => event,
                Err(error) => error,
            };
            self.channel
                .lock()
                .await
                .write_all(message.as_vec().as_slice())
                .await
                .expect("OK");
        });
        Ok(())
    }
}

/// Trait to handle server to client query results for PostgreSQL Wire Protocol
/// connection
pub trait Sender: Send + Sync {
    /// Flushes the output stream.
    fn flush(&self) -> io::Result<()>;

    /// Sends response messages to client. Most of the time it is a single
    /// message, select result one of the exceptional situation
    fn send(&self, query_result: Result<BackendMessage, BackendMessage>) -> io::Result<()>;
}

/// Manages allocation of Connection IDs and secret keys.
#[derive(Clone)]
pub struct ConnSupervisor {
    inner: Arc<Mutex<ConnSupervisorInner>>,
}

impl ConnSupervisor {
    /// Creates a new Connection Supervisor.
    pub fn new(min_id: ConnId, max_id: ConnId) -> ConnSupervisor {
        ConnSupervisor {
            inner: Arc::new(Mutex::new(ConnSupervisorInner::new(min_id, max_id))),
        }
    }

    // TODO: better error type
    /// Allocates a new Connection ID and secret key.
    #[allow(clippy::result_unit_err)]
    pub fn alloc(&self) -> Result<(ConnId, ConnSecretKey), ()> {
        self.inner.lock().unwrap().alloc()
    }

    /// Releases a Connection ID back to the pool.
    pub fn free(&self, conn_id: ConnId) {
        self.inner.lock().unwrap().free(conn_id);
    }

    /// Validates whether the secret key matches the specified Connection ID.
    pub fn verify(&self, conn_id: ConnId, secret_key: ConnSecretKey) -> bool {
        self.inner.lock().unwrap().verify(conn_id, secret_key)
    }
}

struct ConnSupervisorInner {
    next_id: ConnId,
    max_id: ConnId,
    free_ids: VecDeque<ConnId>,
    current_mapping: HashMap<ConnId, ConnSecretKey>,
}

impl ConnSupervisorInner {
    /// Creates a new Connection Supervisor.
    pub fn new(min_id: ConnId, max_id: ConnId) -> ConnSupervisorInner {
        ConnSupervisorInner {
            next_id: min_id,
            max_id,
            free_ids: VecDeque::new(),
            current_mapping: HashMap::new(),
        }
    }

    /// Allocates a new Connection ID and secret key.
    fn alloc(&mut self) -> Result<(ConnId, ConnSecretKey), ()> {
        let conn_id = self.generate_conn_id()?;
        let secret_key = rand::thread_rng().gen();
        self.current_mapping.insert(conn_id, secret_key);
        Ok((conn_id, secret_key))
    }

    /// Releases a Connection ID back to the pool.
    fn free(&mut self, conn_id: ConnId) {
        if self.current_mapping.remove(&conn_id).is_some() {
            self.free_ids.push_back(conn_id);
        }
    }

    /// Validates whether the secret key matches the specified Connection ID.
    fn verify(&self, conn_id: ConnId, secret_key: ConnSecretKey) -> bool {
        match self.current_mapping.get(&conn_id) {
            Some(s) => *s == secret_key,
            None => false,
        }
    }

    fn generate_conn_id(&mut self) -> Result<ConnId, ()> {
        match self.free_ids.pop_front() {
            Some(id) => Ok(id),
            None => {
                let id = self.next_id;
                if id > self.max_id {
                    return Err(());
                }

                self.next_id += 1;
                Ok(id)
            }
        }
    }
}

/// Accepting or Rejecting SSL connection
pub enum Encryption {
    /// Accept SSL connection from client
    AcceptSsl,
    /// Reject SSL connection from client
    RejectSsl,
}

impl From<Encryption> for &'static [u8] {
    fn from(encryption: Encryption) -> &'static [u8] {
        match encryption {
            Encryption::AcceptSsl => &[b'S'],
            Encryption::RejectSsl => &[b'N'],
        }
    }
}

/// Struct to configure possible secure providers for client-server communication
/// PostgreSQL Wire Protocol supports `ssl`/`tls` and `gss` encryption
pub struct ProtocolConfiguration {
    ssl_conf: Option<(PathBuf, String)>,
}

#[allow(dead_code)]
impl ProtocolConfiguration {
    /// Creates configuration that support neither `ssl` nor `gss` encryption
    pub fn none() -> Self {
        Self { ssl_conf: None }
    }

    /// Creates configuration that support only `ssl`
    pub fn with_ssl(cert: PathBuf, password: String) -> Self {
        Self {
            ssl_conf: Some((cert, password)),
        }
    }

    /// returns `true` if support `ssl` connection
    pub fn ssl_support(&self) -> bool {
        self.ssl_conf.is_some()
    }

    /// cert file and its password
    pub fn ssl_config(&self) -> Option<&(PathBuf, String)> {
        self.ssl_conf.as_ref()
    }

    /// returns `true` if support `gss` encrypted connection
    pub fn gssenc_support(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests;
