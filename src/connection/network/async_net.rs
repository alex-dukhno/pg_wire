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

#[cfg(not(feature = "tokio_net"))]
use crate::connection::async_native_tls::{self, TlsStream};
use crate::connection::AcceptError;
use async_io::Async;
use blocking::Unblock;
pub use futures_lite::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::{
    fs::File,
    io,
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

impl From<Async<TcpListener>> for Network {
    fn from(tcp: Async<TcpListener>) -> Network {
        Network { inner: tcp }
    }
}

impl From<Async<TcpStream>> for Stream {
    fn from(tcp: Async<TcpStream>) -> Stream {
        Stream { inner: tcp }
    }
}

impl From<TlsStream<Stream>> for SecureStream {
    fn from(stream: TlsStream<Stream>) -> SecureStream {
        SecureStream { inner: stream }
    }
}

/// Abstracts underling mechanics of establishing connection between client and server
pub struct Network {
    inner: Async<TcpListener>,
}

impl Network {
    /// Accept a new incoming stream from this network.
    pub async fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        self.inner
            .accept()
            .await
            .map(|(stream, addr)| (Stream::from(stream), addr))
    }

    /// Accept a new incoming tls stream from this network.
    pub async fn tls_accept(
        &self,
        certificate_path: &Path,
        password: &str,
        stream: Stream,
    ) -> Result<SecureStream, AcceptError> {
        Ok(SecureStream::from(
            async_native_tls::accept(Unblock::new(File::open(certificate_path)?), password, stream).await?,
        ))
    }
}

pub struct SecureStream {
    inner: TlsStream<Stream>,
}

impl AsyncRead for SecureStream {
    fn poll_read(self: Pin<&mut SecureStream>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
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

    fn poll_close(self: Pin<&mut SecureStream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_close(cx)
    }
}

pub struct Stream {
    inner: Async<TcpStream>,
}

impl AsyncRead for Stream {
    fn poll_read(self: Pin<&mut Stream>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
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

    fn poll_close(self: Pin<&mut Stream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_close(cx)
    }
}

pub enum Channel {
    Plain(Stream),
    Secure(SecureStream),
}

impl AsyncRead for Channel {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
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

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Channel::Plain(tcp) => Pin::new(tcp).poll_close(cx),
            Channel::Secure(tls) => Pin::new(tls).poll_close(cx),
        }
    }
}
