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

#![warn(missing_docs)]
//! API for backend implementation of PostgreSQL Wire Protocol

#[cfg(all(feature = "async_net", feature = "tokio_net"))]
compile_error!("feature \"async_net\" and feature \"tokio_net\" cannot be enabled at the same time");

pub use connection::{
    listener::PgWireListener, network::Network, ClientRequest, ConnSupervisor, ProtocolConfiguration, ResponseSender,
    Sender,
};
pub use errors::{HandShakeError, MessageFormatError, PayloadError};
pub use frontend::CommandMessage;

mod connection;
mod cursor;
mod errors;
mod frontend;
mod hand_shake;
mod message_decoder;
mod request_codes;

/// Connection key-value params
pub type ClientParams = Vec<(String, String)>;
