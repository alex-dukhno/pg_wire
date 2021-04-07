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

use crate::connection::network::{Stream, StreamInner, Network, NetworkInner, SecureStream, SecureStreamInner};
use std::sync::{Arc, Mutex};
use futures_lite::io::{AsyncRead, AsyncWrite};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::io;

pub type TcpListener = TestCase;
pub type TcpStream = TestCase;
pub type TlsStream = TestCase;

impl From<TestCase> for Network {
    fn from(test_case: TestCase) -> Self {
        Network {
            inner: NetworkInner::Mock(test_case),
        }
    }
}

impl From<TestCase> for Stream {
    fn from(test_case: TestCase) -> Stream {
        Stream {
            inner: StreamInner::Mock(test_case),
        }
    }
}

impl From<TestCase> for SecureStream {
    fn from(test_case: TestCase) -> SecureStream {
        SecureStream {
            inner: SecureStreamInner::Mock(test_case),
        }
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
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context, buf: &mut [u8]) -> Poll<io::Result<usize>> {
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
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut case = self.get_mut().inner.lock().unwrap();
        case.write_content.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}