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

use crate::Oid;
use std::fmt::{self, Display, Formatter};

pub use hand_shake_error::*;
pub use message_format_error::*;
pub use payload_error::*;
pub use type_value_decode_error::*;

/// Represents an error if frontend sent unrecognizable format
/// contains the integer code that was sent
#[derive(Debug, PartialEq)]
pub struct UnrecognizedFormat(pub(crate) i16);

impl Display for UnrecognizedFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "unknown format code: {}", self.0)
    }
}

/// Represents an error if frontend sent [Oid] that is not supported
#[derive(Debug, PartialEq)]
pub struct NotSupportedOid(pub(crate) Oid);

impl Display for NotSupportedOid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "OID: '{}' is not supported", self.0)
    }
}

mod hand_shake_error {
    use crate::{errors::PayloadError, request_codes::Code};
    use std::fmt::{self, Display, Formatter};

    /// An error which can be returned during [HandShakeProcess](crate::hand_shake::Process)
    /// and client send erroneous bytes or functionality is not yet supported
    #[derive(Debug, PartialEq)]
    pub struct HandShakeError<'e> {
        kind: HandShakeErrorKind<'e>,
    }

    impl<'e> Display for HandShakeError<'e> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match &self.kind {
                HandShakeErrorKind::UnsupportedProtocolVersion(code) => {
                    write!(f, "Unsupported Protocol Version: {}", code)
                }
                HandShakeErrorKind::UnsupportedClientRequest(code) => {
                    write!(f, "Unsupported Client Code Request: '{}'", code)
                }
                HandShakeErrorKind::PayloadError(error) => write!(f, "{}", error),
            }
        }
    }

    impl<'e> From<HandShakeErrorKind<'e>> for HandShakeError<'e> {
        fn from(kind: HandShakeErrorKind) -> HandShakeError {
            HandShakeError { kind }
        }
    }

    impl<'e> From<PayloadError<'e>> for HandShakeError<'e> {
        fn from(error: PayloadError<'e>) -> HandShakeError {
            HandShakeError {
                kind: HandShakeErrorKind::PayloadError(error),
            }
        }
    }

    #[derive(Debug, PartialEq)]
    pub(crate) enum HandShakeErrorKind<'e> {
        UnsupportedProtocolVersion(Code),
        UnsupportedClientRequest(Code),
        PayloadError(PayloadError<'e>),
    }

    #[cfg(test)]
    mod formatting {
        use super::*;
        use crate::{errors::PayloadErrorKind, request_codes::VERSION_1_CODE};

        #[test]
        fn from_payload_error() {
            assert_eq!(
                HandShakeError::from(PayloadError::from(PayloadErrorKind::EndOfBuffer)).to_string(),
                "End of Payload Buffer"
            );
        }

        #[test]
        fn unsupported_protocol_version() {
            assert_eq!(
                HandShakeError::from(HandShakeErrorKind::UnsupportedProtocolVersion(VERSION_1_CODE)).to_string(),
                "Unsupported Protocol Version: Version 1.0 Request"
            );
        }

        #[test]
        fn unsupported_client_request() {
            assert_eq!(
                HandShakeError::from(HandShakeErrorKind::UnsupportedClientRequest(Code(0x12_34_56_78))).to_string(),
                "Unsupported Client Code Request: 'High bytes 0x1234 Low bytes: 0x5678'"
            );
        }
    }
}

mod message_format_error {
    use crate::errors::{NotSupportedOid, PayloadError, UnrecognizedFormat};
    use std::fmt::{self, Display, Formatter};

    /// An error which can be returned when decoding
    /// [FrontendMessage](crate::messages::FrontendMessage)s from raw bytes
    #[derive(Debug, PartialEq)]
    pub struct MessageFormatError<'e> {
        kind: MessageFormatErrorKind<'e>,
    }

    impl<'e> From<MessageFormatErrorKind<'e>> for MessageFormatError<'e> {
        fn from(kind: MessageFormatErrorKind<'e>) -> MessageFormatError {
            MessageFormatError { kind }
        }
    }

    impl<'e> From<PayloadError<'e>> for MessageFormatError<'e> {
        fn from(error: PayloadError<'e>) -> MessageFormatError {
            MessageFormatError {
                kind: MessageFormatErrorKind::PayloadError(error),
            }
        }
    }

    impl<'e> From<NotSupportedOid> for MessageFormatError<'e> {
        fn from(error: NotSupportedOid) -> MessageFormatError<'e> {
            MessageFormatError {
                kind: MessageFormatErrorKind::NotSupportedOid(error),
            }
        }
    }

    impl<'e> From<UnrecognizedFormat> for MessageFormatError<'e> {
        fn from(error: UnrecognizedFormat) -> MessageFormatError<'e> {
            MessageFormatError {
                kind: MessageFormatErrorKind::UnrecognizedFormat(error),
            }
        }
    }

    impl<'e> Display for MessageFormatError<'e> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match &self.kind {
                MessageFormatErrorKind::PayloadError(error) => write!(f, "{}", error),
                MessageFormatErrorKind::InvalidTypeByte(type_byte) => {
                    write!(f, "invalid type byte in Describe frontend message: '{}'", type_byte)
                }
                MessageFormatErrorKind::UnsupportedFrontendMessage(tag) => {
                    write!(f, "unsupported frontend message tag '{}'", tag)
                }
                MessageFormatErrorKind::NotSupportedOid(error) => write!(f, "{}", error),
                MessageFormatErrorKind::UnrecognizedFormat(error) => write!(f, "{}", error),
                MessageFormatErrorKind::MissingMessageTag => write!(f, "Message tag is not found in payload"),
            }
        }
    }

    #[derive(Debug, PartialEq)]
    pub(crate) enum MessageFormatErrorKind<'e> {
        MissingMessageTag,
        PayloadError(PayloadError<'e>),
        InvalidTypeByte(char),
        UnsupportedFrontendMessage(char),
        NotSupportedOid(NotSupportedOid),
        UnrecognizedFormat(UnrecognizedFormat),
    }

    #[cfg(test)]
    mod formatting {
        use super::*;
        use crate::errors::PayloadErrorKind;

        #[test]
        fn from_payload_error() {
            assert_eq!(
                MessageFormatError::from(PayloadError::from(PayloadErrorKind::EndOfBuffer)).to_string(),
                "End of Payload Buffer"
            );
        }

        #[test]
        fn from_not_supported_oid() {
            assert_eq!(
                MessageFormatError::from(NotSupportedOid(100)).to_string(),
                "OID: '100' is not supported"
            );
        }

        #[test]
        fn from_unrecognized_format() {
            assert_eq!(
                MessageFormatError::from(UnrecognizedFormat(5)).to_string(),
                "unknown format code: 5"
            );
        }

        #[test]
        fn unsupported_frontend_message() {
            assert_eq!(
                MessageFormatError::from(MessageFormatErrorKind::UnsupportedFrontendMessage('t')).to_string(),
                "unsupported frontend message tag 't'"
            );
        }

        #[test]
        fn missing_message_tag() {
            assert_eq!(
                MessageFormatError::from(MessageFormatErrorKind::MissingMessageTag).to_string(),
                "Message tag is not found in payload"
            );
        }

        #[test]
        fn invalid_type_byte() {
            assert_eq!(
                MessageFormatError::from(MessageFormatErrorKind::InvalidTypeByte('U')).to_string(),
                "invalid type byte in Describe frontend message: 'U'"
            );
        }
    }
}

mod type_value_decode_error {
    use crate::types::PgType;
    use std::{
        fmt::{self, Display, Formatter},
        num::ParseIntError,
        str::Utf8Error,
    };

    /// An error which can be returned when decoding [Value](crate::types::Value)s from raw bytes
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

    #[cfg(test)]
    mod formatting {
        use super::*;
        use std::{str, str::FromStr};

        #[test]
        fn not_enough_bytes() {
            assert_eq!(
                TypeValueDecodeError::from(TypeValueDecodeErrorKind::NotEnoughBytes {
                    required_bytes: 8,
                    source: &[0, 0, 1],
                    pg_type: PgType::BigInt,
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
                    source: &[non_utf_code],
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
                    pg_type: PgType::Integer,
                })
                .to_string(),
                "integer type can not be parsed from \'1.0\'. The cause: \"invalid digit found in string\""
            )
        }
    }
}

mod payload_error {
    use std::{
        fmt::{self, Display, Formatter},
        str::Utf8Error,
    };

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

    #[cfg(test)]
    mod formatting {
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
                    source: &buffer,
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
                    source: &buffer,
                })
                .to_string(),
                "Buffer does not contain required number of bytes. Bytes required 4, buffer content [0, 123]"
            );
        }
    }
}
