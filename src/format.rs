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

use std::convert::TryFrom;

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
pub struct UnrecognizedFormat(pub(crate) i16);
