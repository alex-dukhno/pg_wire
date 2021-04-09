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
    connection::{network::*, AcceptError, ClientRequest, ConnSupervisor, Encryption, ProtocolConfiguration},
    hand_shake::{HandShakeProcess, HandShakeStatus},
};
use std::io;

/// A PostgreSql connection server, listening for connections.
pub struct PgWireListener {
    network: Network,
    protocol_config: ProtocolConfiguration,
    conn_supervisor: ConnSupervisor,
}

impl PgWireListener {
    /// creates new PostgreSql connection server
    pub fn new(
        network: Network,
        protocol_config: ProtocolConfiguration,
        conn_supervisor: ConnSupervisor,
    ) -> PgWireListener {
        PgWireListener {
            network,
            protocol_config,
            conn_supervisor,
        }
    }

    /// Accept a new incoming connection from this listener.
    pub async fn accept(&self) -> io::Result<Result<ClientRequest, ()>> {
        match self.network.accept().await {
            Ok((stream, address)) => {
                let mut channel = Channel::Plain(stream);
                let mut process = HandShakeProcess::start();
                let mut current: Option<Vec<u8>> = None;
                loop {
                    match process.next_stage(current.as_deref()) {
                        Ok(HandShakeStatus::RequestingBytes(len)) => {
                            let mut local = vec![b'0'; len];
                            local = channel.read_exact(&mut local).await.map(|_| local)?;
                            current = Some(local);
                        }
                        Ok(HandShakeStatus::UpdatingToSecure) => {
                            channel = match channel {
                                Channel::Plain(mut channel) if self.protocol_config.ssl_support() => {
                                    channel.write_all(Encryption::AcceptSsl.into()).await?;
                                    match self.protocol_config.ssl_config() {
                                        Some((path, password)) => {
                                            match self.network.tls_accept(path, password, channel).await {
                                                Ok(socket) => Channel::Secure(socket),
                                                Err(err) => {
                                                    return match err {
                                                        AcceptError::NativeTls(_tls) => Ok(Err(())),
                                                        AcceptError::Io(io_error) => Err(io_error),
                                                    }
                                                }
                                            }
                                        }
                                        None => return Err(io::Error::from(io::ErrorKind::ConnectionAborted)),
                                    }
                                }
                                _ => {
                                    channel.write_all(Encryption::RejectSsl.into()).await?;
                                    channel
                                }
                            };
                            let mut local = vec![b'0'; 4];
                            local = channel.read_exact(&mut local).await.map(|_| local)?;
                            current = Some(local);
                        }
                        Ok(HandShakeStatus::Cancel(conn_id, secret_key)) => {
                            return if self.conn_supervisor.verify(conn_id, secret_key) {
                                Ok(Ok(ClientRequest::QueryCancellation(conn_id)))
                            } else {
                                Ok(Err(()))
                            }
                        }
                        Ok(HandShakeStatus::Done(props)) => {
                            return Ok(Ok(ClientRequest::Connect2((
                                channel,
                                props,
                                self.conn_supervisor.clone(),
                                address,
                            ))))
                        }
                        Err(_error) => {
                            return Ok(Err(()));
                        }
                    }
                }
            }
            Err(io_error) => Err(io_error),
        }
    }
}
