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

pub use tokio::io::{AsyncRead, AsyncWriteExt, AsyncWrite, AsyncReadExt};
use std::{
    io,
    net::{SocketAddr},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::TlsStream;
use crate::connection::async_native_tls::AcceptError;
use tokio::io::ReadBuf;
use tokio::fs::File;

impl From<TcpListener> for Network {
    fn from(tcp: TcpListener) -> Network {
        Network {
            inner: tcp,
        }
    }
}

impl From<TcpStream> for Stream {
    fn from(tcp: TcpStream) -> Stream {
        Stream {
            inner: tcp,
        }
    }
}

impl From<TlsStream<Stream>> for SecureStream {
    fn from(stream: TlsStream<Stream>) -> SecureStream {
        SecureStream {
            inner: stream,
        }
    }
}

pub struct Network {
    inner: TcpListener,
}

impl Network {
    pub async fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        self.inner.accept().await.map(|(stream, addr)| (Stream::from(stream), addr))
    }

    pub async fn tls_accept(
        &self,
        certificate_path: &PathBuf,
        password: &str,
        stream: Stream,
    ) -> Result<SecureStream, AcceptError> {
        let mut identity = vec![];
        let mut file = File::open(certificate_path).await.map_err(|e| AcceptError::Io(e))?;
        file.read_to_end(&mut identity).await?;

        let identity = native_tls::Identity::from_pkcs12(&identity, password.as_ref())?;
        let acceptor = tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::new(identity)?);
        let tls_stream = acceptor.accept(stream).await?;
        Ok(SecureStream::from(tls_stream))
    }
}

pub struct SecureStream {
    inner: TlsStream<Stream>,
}

impl AsyncRead for SecureStream {
    fn poll_read(self: Pin<&mut SecureStream>, cx: &mut Context<'_>, buf: &mut ReadBuf) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for SecureStream {
    fn poll_write(self: Pin<&mut SecureStream>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut SecureStream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

pub struct Stream {
    inner: TcpStream,
}

impl AsyncRead for Stream {
    fn poll_read(self: Pin<&mut Stream>, cx: &mut Context<'_>, buf: &mut ReadBuf) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(self: Pin<&mut Stream>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Stream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Stream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

pub enum Channel {
    Plain(Stream),
    Secure(SecureStream),
}

impl AsyncRead for Channel {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Channel::Plain(tcp) => Pin::new(tcp).poll_read(cx, buf),
            Channel::Secure(tls) => Pin::new(tls).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Channel {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Channel::Plain(tcp) => Pin::new(tcp).poll_write(cx, buf),
            Channel::Secure(tls) => Pin::new(tls).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Channel::Plain(tcp) => Pin::new(tcp).poll_flush(cx),
            Channel::Secure(tls) => Pin::new(tls).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Channel::Plain(tcp) => Pin::new(tcp).poll_shutdown(cx),
            Channel::Secure(tls) => Pin::new(tls).poll_shutdown(cx),
        }
    }
}
