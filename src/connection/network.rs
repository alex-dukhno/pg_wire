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

#[cfg(feature = "async_net")]
mod async_net;
#[cfg(feature = "mock_network")]
pub(crate) mod mock;

use crate::connection::async_native_tls::AcceptError;
use futures_lite::io::{AsyncRead, AsyncWrite};
use std::{
    io,
    net::{SocketAddr},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "async_net")]
use async_net::*;
#[cfg(feature = "async_net")]
use blocking::Unblock;
#[cfg(feature = "async_net")]
use crate::connection::async_native_tls;
#[cfg(feature = "async_net")]
use std::fs::File;

#[cfg(feature = "mock_network")]
use mock::*;

pub struct Network {
    inner: NetworkInner,
}

impl Network {
    pub async fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        self.inner.accept().await
    }

    pub async fn tls_accept(
        &self,
        certificate_path: &PathBuf,
        password: &str,
        stream: Stream,
    ) -> Result<SecureStream, AcceptError> {
        self.inner.tls_accept(certificate_path, password, stream).await
    }
}

enum NetworkInner {
    #[cfg(feature = "async_net")]
    AsyncNet(TcpListener),
    #[cfg(feature = "mock_network")]
    Mock(TcpListener),
}

impl NetworkInner {
    async fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        match self {
            #[cfg(feature = "async_net")]
            NetworkInner::AsyncNet(tcp) => tcp.accept().await.map(|(stream, addr)| (Stream::from(stream), addr)),
            #[cfg(feature = "mock_network")]
            NetworkInner::Mock(data) => {
                use std::net::{IpAddr, Ipv4Addr};
                Ok((
                    Stream::from(data.clone()),
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1000),
                ))
            }
        }
    }

    async fn tls_accept(
        &self,
        certificate_path: &PathBuf,
        password: &str,
        stream: Stream,
    ) -> Result<SecureStream, AcceptError> {
        match self {
            #[cfg(feature = "async_net")]
            NetworkInner::AsyncNet(_) => Ok(SecureStream::from(
                async_native_tls::accept(Unblock::new(File::open(certificate_path)?), password, stream).await?,
            )),
            #[cfg(feature = "mock_network")]
            NetworkInner::Mock(data) => Ok(SecureStream::from(data.clone())),
        }
    }
}

pub struct SecureStream {
    inner: SecureStreamInner,
}

enum SecureStreamInner {
    #[cfg(feature = "async_net")]
    Tls(TlsStream),
    #[cfg(feature = "mock_network")]
    Mock(TlsStream),
}

impl AsyncRead for SecureStream {
    fn poll_read(self: Pin<&mut SecureStream>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            SecureStreamInner::Tls(tls) => Pin::new(tls).poll_read(cx, buf),
            #[cfg(feature = "mock_network")]
            SecureStreamInner::Mock(data) => Pin::new(data).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for SecureStream {
    fn poll_write(self: Pin<&mut SecureStream>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            SecureStreamInner::Tls(tls) => Pin::new(tls).poll_write(cx, buf),
            #[cfg(feature = "mock_network")]
            SecureStreamInner::Mock(data) => Pin::new(data).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut SecureStream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            SecureStreamInner::Tls(tls) => Pin::new(tls).poll_flush(cx),
            #[cfg(feature = "mock_network")]
            SecureStreamInner::Mock(data) => Pin::new(data).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut SecureStream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            SecureStreamInner::Tls(tls) => Pin::new(tls).poll_close(cx),
            #[cfg(feature = "mock_network")]
            SecureStreamInner::Mock(data) => Pin::new(data).poll_close(cx),
        }
    }
}

pub struct Stream {
    inner: StreamInner,
}

enum StreamInner {
    #[cfg(feature = "async_net")]
    AsyncNet(TcpStream),
    #[cfg(feature = "mock_network")]
    Mock(TcpStream),
}

impl AsyncRead for Stream {
    fn poll_read(self: Pin<&mut Stream>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            StreamInner::AsyncNet(tcp) => Pin::new(tcp).poll_read(cx, buf),
            #[cfg(feature = "mock_network")]
            StreamInner::Mock(data) => Pin::new(data).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(self: Pin<&mut Stream>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            StreamInner::AsyncNet(tcp) => Pin::new(tcp).poll_write(cx, buf),
            #[cfg(feature = "mock_network")]
            StreamInner::Mock(data) => Pin::new(data).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Stream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            StreamInner::AsyncNet(tcp) => Pin::new(tcp).poll_flush(cx),
            #[cfg(feature = "mock_network")]
            StreamInner::Mock(data) => Pin::new(data).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Stream>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "async_net")]
            StreamInner::AsyncNet(tcp) => Pin::new(tcp).poll_close(cx),
            #[cfg(feature = "mock_network")]
            StreamInner::Mock(data) => Pin::new(data).poll_close(cx),
        }
    }
}
