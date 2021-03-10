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

use crate::{format::UnrecognizedFormat, request_codes::Code, Oid, PgType};
use std::{
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    str::Utf8Error,
};

/// An error which can be returned when decoding raw bytes into [Value](crate::types::Value)s
#[derive(Debug, PartialEq)]
pub struct TypeValueDecodeError<'e> {
    kind: TypeValueDecodeErrorKind<'e>,
}

impl<'e> From<TypeValueDecodeErrorKind<'e>> for TypeValueDecodeError<'e> {
    fn from(kind: TypeValueDecodeErrorKind<'e>) -> TypeValueDecodeError<'_> {
        TypeValueDecodeError { kind }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum TypeValueDecodeErrorKind<'e> {
    NotEnoughBytes {
        required_bytes: u8,
        source: &'e [u8],
        pg_type: PgType,
    },
    CannotDecodeString {
        cause: Utf8Error,
        source: &'e [u8],
    },
    CannotParseBool {
        source: &'e str,
    },
    CannotParseInt {
        cause: ParseIntError,
        source: &'e str,
        pg_type: PgType,
    },
}

impl<'e> Display for TypeValueDecodeError<'e> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeValueDecodeErrorKind::NotEnoughBytes {
                required_bytes,
                source,
                pg_type,
            } => write!(
                f,
                "{} type can not be decoded. Its size is {} bytes. Buffer content {:?}",
                pg_type, required_bytes, source
            ),
            TypeValueDecodeErrorKind::CannotDecodeString { cause, source } => {
                write!(
                    f,
                    "UTF-8 string can not be decoded from {:?}. The cause: \"{}\"",
                    source, cause
                )
            }
            TypeValueDecodeErrorKind::CannotParseBool { source } => {
                write!(f, "bool type can not be decoded from '{}'", source)
            }
            TypeValueDecodeErrorKind::CannotParseInt { cause, source, pg_type } => {
                write!(
                    f,
                    "{} type can not be parsed from '{}'. The cause: \"{}\"",
                    pg_type, source, cause
                )
            }
        }
    }
}

/// An error which can be returned when decoding raw bytes into [FrontendMessage](crate::messages::FrontendMessage)s
#[derive(Debug, PartialEq)]
pub struct PayloadError<'e> {
    kind: PayloadErrorKind<'e>,
}

impl<'e> From<PayloadErrorKind<'e>> for PayloadError<'e> {
    fn from(kind: PayloadErrorKind<'e>) -> PayloadError {
        PayloadError { kind }
    }
}

impl<'e> Display for PayloadError<'e> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            PayloadErrorKind::InvalidUtfString { cause, source } => {
                write!(f, "{:?} is invalid UTF-8 string. The cause: \"{}\"", source, cause)
            }
            PayloadErrorKind::CStringNotTerminated { source } => {
                write!(
                    f,
                    "Buffer does not contain \\0 termination byte. Buffer content {:?}",
                    source
                )
            }
            PayloadErrorKind::EndOfBuffer => {
                write!(f, "End of Payload Buffer")
            }
            PayloadErrorKind::NotEnoughBytes { required, source } => {
                write!(
                    f,
                    "Buffer does not contain required number of bytes. Bytes required {}, buffer content {:?}",
                    required, source
                )
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum PayloadErrorKind<'e> {
    InvalidUtfString { cause: Utf8Error, source: &'e [u8] },
    CStringNotTerminated { source: &'e [u8] },
    EndOfBuffer,
    NotEnoughBytes { required: u8, source: &'e [u8] },
}

// temporal WA while API is changing
impl<'e> From<PayloadError<'e>> for Error {
    fn from(error: PayloadError<'_>) -> Self {
        match error.kind {
            PayloadErrorKind::InvalidUtfString { .. } => Error::InvalidUtfString,
            PayloadErrorKind::CStringNotTerminated { .. } => Error::ZeroByteNotFound,
            PayloadErrorKind::EndOfBuffer => Error::InvalidInput("No byte to read".to_owned()),
            PayloadErrorKind::NotEnoughBytes { .. } => {
                Error::InvalidInput("not enough buffer to read 32bit Int".to_owned())
            }
        }
    }
}

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
    /// Indicates that connection verification is failed
    VerificationFailed,
}

impl From<UnrecognizedFormat> for Error {
    fn from(error: UnrecognizedFormat) -> Error {
        Error::InvalidInput(format!("unknown format code: {}", error.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod error_conversion {
        use super::*;

        #[test]
        fn from_unrecognized_format() {
            assert_eq!(
                Error::from(UnrecognizedFormat(100)),
                Error::InvalidInput("unknown format code: 100".to_owned())
            );
        }
    }

    #[cfg(test)]
    mod payload_error {
        use super::*;
        use std::str;

        #[test]
        fn invalid_utf_string() {
            let invalid_utf_byte = 0x96;
            let mut buffer = b"some string".to_vec();
            buffer.push(invalid_utf_byte);
            assert_eq!(
                PayloadError::from(PayloadErrorKind::InvalidUtfString {
                    cause: str::from_utf8(&buffer).unwrap_err(),
                    source: &buffer
                }).to_string(), "[115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103, 150] is invalid UTF-8 string. The cause: \"invalid utf-8 sequence of 1 bytes from index 11\"")
        }

        #[test]
        fn c_string_not_terminated() {
            let buffer = b"some string";
            assert_eq!(
                PayloadError::from(PayloadErrorKind::CStringNotTerminated {
                    source: buffer
                }).to_string(), "Buffer does not contain \\0 termination byte. Buffer content [115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103]"
            );
        }

        #[test]
        fn end_of_buffer() {
            assert_eq!(
                PayloadError::from(PayloadErrorKind::EndOfBuffer).to_string(),
                "End of Payload Buffer"
            )
        }

        #[test]
        fn not_enough_bytes() {
            let buffer = 123i16.to_be_bytes().to_vec();
            assert_eq!(
                PayloadError::from(PayloadErrorKind::NotEnoughBytes {
                    required: 4,
                    source: &buffer
                })
                .to_string(),
                "Buffer does not contain required number of bytes. Bytes required 4, buffer content [0, 123]"
            );
        }
    }

    #[cfg(test)]
    mod type_value_decode_error {
        use super::*;
        use std::{str, str::FromStr};

        #[test]
        fn not_enough_bytes() {
            assert_eq!(
                TypeValueDecodeError::from(TypeValueDecodeErrorKind::NotEnoughBytes {
                    required_bytes: 8,
                    source: &[0, 0, 1],
                    pg_type: PgType::BigInt
                })
                .to_string(),
                "bigint type can not be decoded. Its size is 8 bytes. Buffer content [0, 0, 1]"
            )
        }

        #[test]
        fn can_not_decode_string() {
            let non_utf_code = 0x96;
            assert_eq!(TypeValueDecodeError::from(
                TypeValueDecodeErrorKind::CannotDecodeString {
                    cause: str::from_utf8(&[non_utf_code]).unwrap_err(),
                    source: &[non_utf_code]
                }
            ).to_string(), "UTF-8 string can not be decoded from [150]. The cause: \"invalid utf-8 sequence of 1 bytes from index 0\"")
        }

        #[test]
        fn can_not_parse_bool() {
            assert_eq!(
                TypeValueDecodeError::from(TypeValueDecodeErrorKind::CannotParseBool { source: "abc" }).to_string(),
                "bool type can not be decoded from 'abc'"
            )
        }

        #[test]
        fn can_not_parse_integer() {
            assert_eq!(
                TypeValueDecodeError::from(TypeValueDecodeErrorKind::CannotParseInt {
                    cause: i32::from_str("1.0").unwrap_err(),
                    source: &"1.0",
                    pg_type: PgType::Integer
                })
                .to_string(),
                "integer type can not be parsed from \'1.0\'. The cause: \"invalid digit found in string\""
            )
        }
    }
}
