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

use async_io::Async;
use std::net;
use crate::connection::network::{Stream, StreamInner, Network, NetworkInner, SecureStream, SecureStreamInner};
use crate::connection::async_native_tls;

pub type TcpListener = Async<net::TcpListener>;
pub type TcpStream = Async<net::TcpStream>;
pub type TlsStream = async_native_tls::TlsStream<Stream>;

impl From<TcpListener> for Network {
    fn from(tcp: TcpListener) -> Self {
        Network {
            inner: NetworkInner::AsyncNet(tcp),
        }
    }
}

impl From<TcpStream> for Stream {
    fn from(tcp: TcpStream) -> Stream {
        Stream {
            inner: StreamInner::AsyncNet(tcp),
        }
    }
}

impl From<TlsStream> for SecureStream {
    fn from(stream: TlsStream) -> SecureStream {
        SecureStream {
            inner: SecureStreamInner::Tls(stream),
        }
    }
}
