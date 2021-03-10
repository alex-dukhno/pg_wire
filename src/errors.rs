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

use crate::{format::UnrecognizedFormat, request_codes::Code, Oid};
use std::num::ParseIntError;

/// `Error` type in protocol `Result`. Indicates that something went not well
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Indicates that not supported `Oid` was used to transfer info
    NotSupportedOid(Oid),
    /// Indicates that incoming data is invalid
    InvalidInput(String),
    /// Indicates that incoming data can't be parsed as UTF-8 string
    InvalidUtfString,
    /// Indicates that incoming string is not terminated by zero byte
    ZeroByteNotFound,
    /// Indicates that frontend message is not supported
    UnsupportedFrontendMessage,
    /// Indicates that protocol version is not supported
    UnsupportedVersion(Code),
    /// Indicates that client request is not supported
    UnsupportedRequest(Code),
    /// Indicates that during handshake client sent unrecognized protocol version
    UnrecognizedVersion,
    /// Indicates that connection verification is failed
    VerificationFailed,
}

impl From<UnrecognizedFormat> for Error {
    fn from(error: UnrecognizedFormat) -> Error {
        Error::InvalidInput(format!("unknown format code: {}", error.0))
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Self {
        Error::InvalidInput(error.to_string())
    }
}

#[cfg(test)]
mod error_conversion {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn from_unrecognized_format() {
        assert_eq!(
            Error::from(UnrecognizedFormat(100)),
            Error::InvalidInput("unknown format code: 100".to_owned())
        );
    }

    #[test]
    fn from_parse_int_error() {
        assert_eq!(
            Error::from(i32::from_str("1.2").unwrap_err()),
            Error::InvalidInput("invalid digit found in string".to_owned())
        );
    }
}
