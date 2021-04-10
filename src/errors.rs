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

pub(crate) use hand_shake_error::*;
pub(crate) use message_format_error::*;
pub(crate) use payload_error::*;
use std::fmt::{self, Display, Formatter};

/// Protocol Error
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind
}

#[derive(Debug)]
enum ErrorKind {
    HandShake(HandShakeError),
    MessageFormat(MessageFormatError),
    TlsHandShake(native_tls::Error),
    SecretKeysHaveNotMatch
}

impl From<HandShakeError> for Error {
    fn from(error: HandShakeError) -> Error {
        Error {
            kind: ErrorKind::HandShake(error)
        }
    }
}

impl From<MessageFormatError> for Error {
    fn from(error: MessageFormatError) -> Error {
        Error {
            kind: ErrorKind::MessageFormat(error)
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(error: native_tls::Error) -> Error {
        Error {
            kind: ErrorKind::TlsHandShake(error)
        }
    }
}

impl Error {
    pub(crate) fn secret_keys_have_not_matched() -> Error {
        Error { kind: ErrorKind::SecretKeysHaveNotMatch }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::HandShake(error) => write!(f, "{}", error),
            ErrorKind::MessageFormat(error) => write!(f, "{}", error),
            ErrorKind::TlsHandShake(error) => write!(f, "{}", error),
            ErrorKind::SecretKeysHaveNotMatch => write!(f, "secret for query cancellation has not matched secret of the current connection")
        }
    }
}

mod hand_shake_error {
    use crate::{errors::PayloadError, request_codes::Code};
    use std::fmt::{self, Display, Formatter};

    /// An error which can be returned during [HandShakeProcess](crate::hand_shake::Process)
    /// and client send erroneous bytes or functionality is not yet supported
    #[derive(Debug, PartialEq)]
    pub struct HandShakeError {
        kind: HandShakeErrorKind,
    }

    impl<'e> Display for HandShakeError {
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

    impl<'e> From<HandShakeErrorKind> for HandShakeError {
        fn from(kind: HandShakeErrorKind) -> HandShakeError {
            HandShakeError { kind }
        }
    }

    impl<'e> From<PayloadError> for HandShakeError {
        fn from(error: PayloadError) -> HandShakeError {
            HandShakeError {
                kind: HandShakeErrorKind::PayloadError(error),
            }
        }
    }

    #[derive(Debug, PartialEq)]
    pub(crate) enum HandShakeErrorKind {
        UnsupportedProtocolVersion(Code),
        UnsupportedClientRequest(Code),
        PayloadError(PayloadError),
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
    use pg_wire_payload::{NotSupportedOid, UnrecognizedFormat};
    use std::fmt::{self, Display, Formatter};
    use crate::errors::PayloadError;

    /// An error which can be returned when decoding
    /// [FrontendMessage](crate::messages::FrontendMessage)s from raw bytes
    #[derive(Debug, PartialEq)]
    pub struct MessageFormatError {
        kind: MessageFormatErrorKind,
    }

    impl From<MessageFormatErrorKind> for MessageFormatError {
        fn from(kind: MessageFormatErrorKind) -> MessageFormatError {
            MessageFormatError { kind }
        }
    }

    impl From<PayloadError> for MessageFormatError {
        fn from(error: PayloadError) -> MessageFormatError {
            MessageFormatError {
                kind: MessageFormatErrorKind::PayloadError(error),
            }
        }
    }

    impl<'e> From<NotSupportedOid> for MessageFormatError {
        fn from(error: NotSupportedOid) -> MessageFormatError {
            MessageFormatError {
                kind: MessageFormatErrorKind::NotSupportedOid(error),
            }
        }
    }

    impl<'e> From<UnrecognizedFormat> for MessageFormatError {
        fn from(error: UnrecognizedFormat) -> MessageFormatError {
            MessageFormatError {
                kind: MessageFormatErrorKind::UnrecognizedFormat(error),
            }
        }
    }

    impl<'e> Display for MessageFormatError {
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
    pub(crate) enum MessageFormatErrorKind {
        MissingMessageTag,
        PayloadError(PayloadError),
        InvalidTypeByte(char),
        UnsupportedFrontendMessage(char),
        NotSupportedOid(NotSupportedOid),
        UnrecognizedFormat(UnrecognizedFormat),
    }

    #[cfg(test)]
    mod formatting {
        use super::*;
        use crate::errors::PayloadErrorKind;
        use pg_wire_payload::{PgFormat, PgType};
        use std::convert::TryFrom;

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
                MessageFormatError::from(PgType::from_oid(100).unwrap_err()).to_string(),
                "OID: '100' is not supported"
            );
        }

        #[test]
        fn from_unrecognized_format() {
            assert_eq!(
                MessageFormatError::from(PgFormat::try_from(5).unwrap_err()).to_string(),
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

mod payload_error {
    use std::{
        fmt::{self, Display, Formatter},
        str::Utf8Error,
    };

    /// An error which can be returned when decoding raw bytes into [FrontendMessage](crate::messages::FrontendMessage)s
    #[derive(Debug, PartialEq)]
    pub struct PayloadError {
        kind: PayloadErrorKind,
    }

    impl From<PayloadErrorKind> for PayloadError {
        fn from(kind: PayloadErrorKind) -> PayloadError {
            PayloadError { kind }
        }
    }

    impl<'e> Display for PayloadError {
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
    pub(crate) enum PayloadErrorKind {
        InvalidUtfString { cause: Utf8Error, source: Vec<u8> },
        CStringNotTerminated { source: Vec<u8> },
        EndOfBuffer,
        NotEnoughBytes { required: u8, source: Vec<u8> },
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
                    source: buffer.to_vec(),
                }).to_string(), "[115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103, 150] is invalid UTF-8 string. The cause: \"invalid utf-8 sequence of 1 bytes from index 11\"")
        }

        #[test]
        fn c_string_not_terminated() {
            let buffer = b"some string";
            assert_eq!(
                PayloadError::from(PayloadErrorKind::CStringNotTerminated {
                    source: buffer.to_vec()
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
                    source: buffer.to_vec(),
                })
                .to_string(),
                "Buffer does not contain required number of bytes. Bytes required 4, buffer content [0, 123]"
            );
        }
    }
}
