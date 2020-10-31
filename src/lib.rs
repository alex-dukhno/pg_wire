// Copyright 2020 Alex Dukhno
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

use std::{convert::TryFrom, fmt::Debug};

pub use hand_shake::{Process as HandShakeProcess, Request as HandShakeRequest, Status as HandShakeStatus};
pub use message_decoder::{MessageDecoder, Status as MessageDecoderStatus};
pub use messages::{BackendMessage, ColumnMetadata, FrontendMessage};

mod cursor;
mod hand_shake;
mod message_decoder;
/// Module contains backend messages that could be send by server implementation
/// to a client
mod messages;
mod request_codes;
mod types;

/// Connection key-value params
pub type ClientParams = Vec<(String, String)>;
/// Protocol operation result
pub type Result<T> = std::result::Result<T, Error>;

/// PostgreSQL OID [Object Identifier](https://www.postgresql.org/docs/current/datatype-oid.html)
pub type Oid = u32;
/// Connection ID
pub(crate) type ConnId = i32;
/// Connection secret key
pub(crate) type ConnSecretKey = i32;

/// PostgreSQL formats for transferring data
/// `0` - textual representation
/// `1` - binary representation
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PgFormat {
    /// data from/to client should be sent in text format
    Text,
    /// data from/to client should be sent in binary format
    Binary,
}

impl TryFrom<i16> for PgFormat {
    type Error = UnrecognizedFormat;

    fn try_from(value: i16) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(PgFormat::Text),
            1 => Ok(PgFormat::Binary),
            other => Err(UnrecognizedFormat(other)),
        }
    }
}

/// Represents an error if frontend sent unrecognizable format
/// contains the integer code that was sent
#[derive(Debug)]
pub struct UnrecognizedFormat(i16);

/// `Error` type in protocol `Result`. Indicates that something went not well
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Indicates that the current count of active connections is full
    ConnectionIdExhausted,
    /// Indicates that incoming data is invalid
    InvalidInput(String),
    /// Indicates that incoming data can't be parsed as UTF-8 string
    InvalidUtfString,
    /// Indicates that incoming string is not terminated by zero byte
    ZeroByteNotFound,
    /// Indicates that frontend message is not supported
    UnsupportedFrontendMessage,
    /// Indicates that protocol version is not supported
    UnsupportedVersion,
    /// Indicates that client request is not supported
    UnsupportedRequest,
    /// Indicates that during handshake client sent unrecognized protocol version
    UnrecognizedVersion,
    /// Indicates that connection verification is failed
    VerificationFailed,
}
