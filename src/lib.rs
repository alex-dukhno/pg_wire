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

pub use errors::{HandShakeError, MessageFormatError, PayloadError, TypeValueDecodeError};
pub use format::PgFormat;
pub use frontend::{CommandMessage, HandShakeMessage};
pub use hand_shake::{Process as HandShakeProcess, Request as HandShakeRequest, Status as HandShakeStatus};
pub use message_decoder::{MessageDecoder, Status as MessageDecoderStatus};
pub use messages::{BackendMessage, ColumnMetadata};
pub use types::{PgType, Value};

mod cursor;
mod errors;
mod format;
mod frontend;
mod hand_shake;
mod message_decoder;
mod messages;
mod request_codes;
mod types;

/// Connection key-value params
pub type ClientParams = Vec<(String, String)>;

/// PostgreSQL OID [Object Identifier](https://www.postgresql.org/docs/current/datatype-oid.html)
pub type Oid = u32;
/// Connection ID
pub type ConnId = i32;
/// Connection secret key
pub type ConnSecretKey = i32;
