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

use crate::{
    cursor::Cursor,
    errors::{MessageFormatError, MessageFormatErrorKind},
    messages::{FrontendMessage, BIND, CLOSE, DESCRIBE, EXECUTE, FLUSH, PARSE, QUERY, SYNC, TERMINATE},
    PgFormat, PgType,
};
use std::convert::TryFrom;

/// Represents a status of a `MessageDecoder` stage
#[derive(Debug, PartialEq)]
pub enum Status {
    /// `MessageDecoder` requests buffer with specified size
    Requesting(usize),
    /// `MessageDecoder` has decoded a message and returns it content
    Done(FrontendMessage),
}

#[derive(Debug, PartialEq)]
pub(crate) enum State {
    RequestingTag,
    Tag(u8),
    WaitingForPayload,
}

/// Decodes messages from client
///
/// # Examples
///
/// ```ignore
/// use pg_wire::{MessageDecoder, MessageDecoderStatus};
///
/// let mut message_decoder = MessageDecoder::new();
/// let mut current: Option<Vec<u8>> = None;
/// loop {
///     log::debug!("Read bytes from connection {:?}", current);
///     match message_decoder.next_stage(current.take().as_deref()) {
///         Ok(MessageDecoderStatus::Requesting(len)) => {
///             let mut buffer = vec![b'0'; len];
///             channel.read_exact(&mut buffer)?;
///             current = Some(buffer);
///         }
///         Ok(MessageDecoderStatus::Done(message)) => return Ok(Ok(message)),
///         Err(error) => return Err(error),
///     }
/// }
/// ```
#[derive(Default)]
pub struct MessageDecoder {
    state: Option<State>,
    tag: u8,
}

impl MessageDecoder {
    /// Proceed to the next stage of decoding received message
    pub fn next_stage<'e>(&'e mut self, payload: Option<&'e [u8]>) -> Result<Status, MessageFormatError<'e>> {
        let buf = if let Some(payload) = payload { payload } else { &[] };
        match self.state.take() {
            None => {
                self.state = Some(State::RequestingTag);
                Ok(Status::Requesting(1))
            }
            Some(State::RequestingTag) => {
                if buf.is_empty() {
                    Err(MessageFormatError::from(MessageFormatErrorKind::MissingMessageTag))
                } else {
                    self.state = Some(State::Tag(buf[0]));
                    Ok(Status::Requesting(4))
                }
            }
            Some(State::Tag(tag)) => {
                self.tag = tag;
                self.state = Some(State::WaitingForPayload);
                Ok(Status::Requesting((Cursor::from(buf).read_i32()? - 4) as usize))
            }
            Some(State::WaitingForPayload) => {
                let message = Self::decode(self.tag, buf)?;
                Ok(Status::Done(message))
            }
        }
    }

    fn decode(tag: u8, buffer: &[u8]) -> Result<FrontendMessage, MessageFormatError> {
        log::debug!("Receives frontend tag = {:?}, buffer = {:?}", char::from(tag), buffer);

        let mut cursor = Cursor::from(buffer);
        match tag {
            // Simple query flow.
            QUERY => {
                let sql = cursor.read_cstr()?.to_owned();
                Ok(FrontendMessage::Query { sql })
            }

            // Extended query flow.
            BIND => {
                let portal_name = cursor.read_cstr()?.to_owned();
                let statement_name = cursor.read_cstr()?.to_owned();

                let mut param_formats = vec![];
                for _ in 0..cursor.read_i16()? {
                    param_formats.push(PgFormat::try_from(cursor.read_i16()?)?)
                }

                let mut raw_params = vec![];
                for _ in 0..cursor.read_i16()? {
                    let len = cursor.read_i32()?;
                    if len == -1 {
                        // As a special case, -1 indicates a NULL parameter value.
                        raw_params.push(None);
                    } else {
                        let mut value = vec![];
                        for _ in 0..len {
                            value.push(cursor.read_byte()?);
                        }
                        raw_params.push(Some(value));
                    }
                }

                let mut result_formats = vec![];
                for _ in 0..cursor.read_i16()? {
                    result_formats.push(PgFormat::try_from(cursor.read_i16()?)?)
                }

                Ok(FrontendMessage::Bind {
                    portal_name,
                    statement_name,
                    param_formats,
                    raw_params,
                    result_formats,
                })
            }
            CLOSE => {
                let first_char = cursor.read_byte()?;
                let name = cursor.read_cstr()?.to_owned();
                match first_char {
                    b'P' => Ok(FrontendMessage::ClosePortal { name }),
                    b'S' => Ok(FrontendMessage::CloseStatement { name }),
                    other => Err(MessageFormatError::from(MessageFormatErrorKind::InvalidTypeByte(
                        char::from(other),
                    ))),
                }
            }
            DESCRIBE => {
                let first_char = cursor.read_byte()?;
                let name = cursor.read_cstr()?.to_owned();
                match first_char {
                    b'P' => Ok(FrontendMessage::DescribePortal { name }),
                    b'S' => Ok(FrontendMessage::DescribeStatement { name }),
                    other => Err(MessageFormatError::from(MessageFormatErrorKind::InvalidTypeByte(
                        char::from(other),
                    ))),
                }
            }
            EXECUTE => {
                let portal_name = cursor.read_cstr()?.to_owned();
                let max_rows = cursor.read_i32()?;
                Ok(FrontendMessage::Execute { portal_name, max_rows })
            }
            FLUSH => Ok(FrontendMessage::Flush),
            PARSE => {
                let statement_name = cursor.read_cstr()?.to_owned();
                let sql = cursor.read_cstr()?.to_owned();

                let mut param_types = vec![];
                for _ in 0..cursor.read_i16()? {
                    let pg_type = PgType::from_oid(cursor.read_u32()?)?;
                    log::trace!("received parameter of {:?} PostgreSQL Type", pg_type);
                    param_types.push(pg_type);
                }

                Ok(FrontendMessage::Parse {
                    statement_name,
                    sql,
                    param_types,
                })
            }
            SYNC => Ok(FrontendMessage::Sync),

            TERMINATE => Ok(FrontendMessage::Terminate),

            _ => Err(MessageFormatError::from(
                MessageFormatErrorKind::UnsupportedFrontendMessage(char::from(tag)),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERY_STRING: &str = "select * from t\0";
    const QUERY_BYTES: &[u8] = QUERY_STRING.as_bytes();
    const LEN: i32 = QUERY_STRING.len() as i32;

    #[cfg(test)]
    mod message_decoder_state_machine {
        use super::*;

        #[test]
        fn request_message_tag() {
            let mut decoder = MessageDecoder::default();

            assert_eq!(decoder.next_stage(None), Ok(Status::Requesting(1)));
        }

        #[test]
        fn request_message_len() {
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            assert_eq!(decoder.next_stage(Some(&[QUERY])), Ok(Status::Requesting(4)));
        }

        #[test]
        fn request_message_payload() {
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[QUERY])).expect("proceed to the next stage");
            assert_eq!(
                decoder.next_stage(Some(&LEN.to_be_bytes())),
                Ok(Status::Requesting((LEN - 4) as usize))
            );
        }

        #[test]
        fn decoding_message() {
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[QUERY])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(QUERY_BYTES)),
                Ok(Status::Done(FrontendMessage::Query {
                    sql: "select * from t".to_owned()
                }))
            );
        }

        #[test]
        fn full_cycle() {
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[QUERY])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            decoder
                .next_stage(Some(QUERY_BYTES))
                .expect("proceed to the next stage");

            assert_eq!(decoder.next_stage(None), Ok(Status::Requesting(1)));
        }
    }

    #[cfg(test)]
    mod decoding_frontend_messages {
        use super::*;

        #[test]
        fn query() {
            let buffer = [
                99, 114, 101, 97, 116, 101, 32, 115, 99, 104, 101, 109, 97, 32, 115, 99, 104, 101, 109, 97, 95, 110,
                97, 109, 101, 59, 0,
            ];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[QUERY])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Query {
                    sql: "create schema schema_name;".to_owned()
                }))
            );
        }

        #[test]
        fn bind() {
            let buffer = [
                112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0, 115, 116, 97, 116, 101, 109, 101, 110, 116, 95,
                110, 97, 109, 101, 0, 0, 2, 0, 1, 0, 1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0,
            ];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[BIND])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Bind {
                    portal_name: "portal_name".to_owned(),
                    statement_name: "statement_name".to_owned(),
                    param_formats: vec![PgFormat::Binary, PgFormat::Binary],
                    raw_params: vec![Some(vec![0, 0, 0, 1]), Some(vec![0, 0, 0, 2])],
                    result_formats: vec![],
                }))
            );
        }

        #[test]
        fn close_portal() {
            let buffer = [80, 112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[CLOSE])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::ClosePortal {
                    name: "portal_name".to_owned(),
                }))
            );
        }

        #[test]
        fn close_statement() {
            let buffer = [83, 115, 116, 97, 116, 101, 109, 101, 110, 116, 95, 110, 97, 109, 101, 0];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[CLOSE])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::CloseStatement {
                    name: "statement_name".to_owned(),
                }))
            );
        }

        #[test]
        fn describe_portal() {
            let buffer = [80, 112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&[DESCRIBE]))
                .expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::DescribePortal {
                    name: "portal_name".to_owned()
                }))
            );
        }

        #[test]
        fn describe_statement() {
            let buffer = [83, 115, 116, 97, 116, 101, 109, 101, 110, 116, 95, 110, 97, 109, 101, 0];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&[DESCRIBE]))
                .expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::DescribeStatement {
                    name: "statement_name".to_owned()
                }))
            );
        }

        #[test]
        fn execute() {
            let buffer = [112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0, 0, 0, 0, 0];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[EXECUTE])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Execute {
                    portal_name: "portal_name".to_owned(),
                    max_rows: 0,
                }))
            );
        }

        #[test]
        fn flush() {
            let buffer = [];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[FLUSH])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Flush))
            );
        }

        #[test]
        fn parse() {
            let buffer = [
                0, 115, 101, 108, 101, 99, 116, 32, 42, 32, 102, 114, 111, 109, 32, 115, 99, 104, 101, 109, 97, 95,
                110, 97, 109, 101, 46, 116, 97, 98, 108, 101, 95, 110, 97, 109, 101, 32, 119, 104, 101, 114, 101, 32,
                115, 105, 95, 99, 111, 108, 117, 109, 110, 32, 61, 32, 36, 49, 59, 0, 0, 1, 0, 0, 0, 23,
            ];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[PARSE])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Parse {
                    statement_name: "".to_owned(),
                    sql: "select * from schema_name.table_name where si_column = $1;".to_owned(),
                    param_types: vec![Some(PgType::Integer)],
                }))
            );
        }

        #[test]
        fn sync() {
            let buffer = [];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder.next_stage(Some(&[SYNC])).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Sync))
            );
        }

        #[test]
        fn terminate() {
            let buffer = [];
            let mut decoder = MessageDecoder::default();

            decoder.next_stage(None).expect("proceed to the next stage");
            decoder
                .next_stage(Some(&[TERMINATE]))
                .expect("proceed to the next stage");
            decoder
                .next_stage(Some(&LEN.to_be_bytes()))
                .expect("proceed to the next stage");

            assert_eq!(
                decoder.next_stage(Some(&buffer)),
                Ok(Status::Done(FrontendMessage::Terminate))
            );
        }
    }
}
