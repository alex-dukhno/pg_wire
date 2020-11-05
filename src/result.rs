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

use crate::types::NotSupportedOid;

/// Protocol operation result
pub type Result<T> = std::result::Result<T, Error>;

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

impl From<NotSupportedOid> for Error {
    fn from(error: NotSupportedOid) -> Error {
        Error::InvalidInput(error.to_string())
    }
}
