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

use crate::connection::AcceptError;
pub use futures_lite::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::{
    io,
    net::SocketAddr,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

impl From<TestCase> for Network {
    fn from(test_case: TestCase) -> Network {
        Network { data: test_case }
    }
}

impl From<TestCase> for Stream {
    fn from(test_case: TestCase) -> Stream {
        Stream { inner: test_case }
    }
}

impl From<TestCase> for SecureStream {
    fn from(test_case: TestCase) -> SecureStream {
        SecureStream { inner: test_case }
    }
}

#[derive(Debug)]
struct TestCaseInner {
    read_content: Vec<u8>,
    read_index: usize,
    write_content: Vec<u8>,
    write_index: usize,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    inner: Arc<Mutex<TestCaseInner>>,
}

impl TestCase {
    pub fn new(content: Vec<&[u8]>) -> TestCase {
        TestCase {
            inner: Arc::new(Mutex::new(TestCaseInner {
                read_content: content.concat(),
                read_index: 0,
                write_content: vec![],
                write_index: 0,
            })),
        }
    }

    pub async fn read_result(&self) -> Vec<u8> {
        self.inner.lock().unwrap().write_content.clone()
    }
}

impl AsyncRead for TestCase {
    fn poll_read(self: Pin<&mut TestCase>, _cx: &mut Context, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let mut case = self.get_mut().inner.lock().unwrap();
        if buf.len() > case.read_content.len() - case.read_index {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::UnexpectedEof)))
        } else {
            for (i, item) in buf.iter_mut().enumerate() {
                *item = case.read_content[case.read_index + i];
            }
            case.read_index += buf.len();
            Poll::Ready(Ok(buf.len()))
        }
    }
}

impl AsyncWrite for TestCase {
    fn poll_write(self: Pin<&mut TestCase>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut case = self.get_mut().inner.lock().unwrap();
        case.write_content.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut TestCase>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut TestCase>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[doc(hidden)]
pub struct Network {
    data: TestCase,
}

impl Network {
    #[doc(hidden)]
    pub async fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        use std::net::{IpAddr, Ipv4Addr};
        Ok((
            Stream::from(self.data.clone()),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1000),
        ))
    }

    #[doc(hidden)]
    pub async fn tls_accept(
        &self,
        _certificate_path: &Path,
        _password: &str,
        _stream: Stream,
    ) -> Result<SecureStream, AcceptError> {
        Ok(SecureStream::from(self.data.clone()))
    }
}

pub struct SecureStream {
    inner: TestCase,
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
    inner: TestCase,
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
