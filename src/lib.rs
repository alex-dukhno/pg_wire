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

pub use format::{PgFormat, UnrecognizedFormat};
pub use hand_shake::{Process as HandShakeProcess, Request as HandShakeRequest, Status as HandShakeStatus};
pub use message_decoder::{MessageDecoder, Status as MessageDecoderStatus};
pub use messages::{BackendMessage, ColumnMetadata, FrontendMessage};
pub use result::{Error, Result};

mod cursor;
mod format;
mod hand_shake;
mod message_decoder;
/// Module contains backend messages that could be send by server implementation
/// to a client
mod messages;
mod request_codes;
mod result;
mod types;

/// Connection key-value params
pub type ClientParams = Vec<(String, String)>;

/// PostgreSQL OID [Object Identifier](https://www.postgresql.org/docs/current/datatype-oid.html)
pub type Oid = u32;
/// Connection ID
pub(crate) type ConnId = i32;
/// Connection secret key
pub(crate) type ConnSecretKey = i32;
