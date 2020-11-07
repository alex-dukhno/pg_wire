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

use crate::{cursor::Cursor, types::PgType, ConnId, ConnSecretKey, Error, PgFormat, Result, UnrecognizedFormat};
use std::convert::TryFrom;

const COMMAND_COMPLETE: u8 = b'C';
const DATA_ROW: u8 = b'D';
const ERROR_RESPONSE: u8 = b'E';
const SEVERITY: u8 = b'S';
const CODE: u8 = b'C';
const MESSAGE: u8 = b'M';
const EMPTY_QUERY_RESPONSE: u8 = b'I';
const NOTICE_RESPONSE: u8 = b'N';
const AUTHENTICATION: u8 = b'R';
const BACKEND_KEY_DATA: u8 = b'K';
const PARAMETER_STATUS: u8 = b'S';
const ROW_DESCRIPTION: u8 = b'T';
const READY_FOR_QUERY: u8 = b'Z';
const PARAMETER_DESCRIPTION: u8 = b't';
const NO_DATA: u8 = b'n';
const PARSE_COMPLETE: u8 = b'1';
const BIND_COMPLETE: u8 = b'2';
const CLOSE_COMPLETE: u8 = b'3';

pub(crate) const QUERY: u8 = b'Q';
const BIND: u8 = b'B';
const CLOSE: u8 = b'C';
const DESCRIBE: u8 = b'D';
const EXECUTE: u8 = b'E';
const FLUSH: u8 = b'H';
const PARSE: u8 = b'P';
const SYNC: u8 = b'S';
const TERMINATE: u8 = b'X';

/// Frontend PostgreSQL Wire Protocol messages
/// see [Protocol Flow](https://www.postgresql.org/docs/current/protocol-flow.html)
/// PostgreSQL documentation section
#[derive(Debug, PartialEq)]
pub enum FrontendMessage {
    /// Client requested GSSENC Request
    GssencRequest,
    /// Client requested SSL connection
    SslRequest,
    /// Connection setup message
    Setup {
        /// client parameters
        params: Vec<(String, String)>,
    },
    /// Execute the specified SQL.
    ///
    /// This is issued as part of the simple query flow.
    Query {
        /// The SQL to execute.
        sql: String,
    },

    /// Parse the specified SQL into a prepared statement.
    ///
    /// This starts the extended query flow.
    Parse {
        /// The name of the prepared statement to create. An empty string
        /// specifies the unnamed prepared statement.
        statement_name: String,
        /// The SQL to parse.
        sql: String,
        /// The number of specified parameter data types can be less than the
        /// number of parameters specified in the query.
        param_types: Vec<Option<PgType>>,
    },

    /// Describe an existing prepared statement.
    ///
    /// This command is part of the extended query flow.
    DescribeStatement {
        /// The name of the prepared statement to describe.
        name: String,
    },

    /// Describe an existing portal.
    ///
    /// This command is part of the extended query flow.
    DescribePortal {
        /// The name of the portal to describe.
        name: String,
    },

    /// Bind an existing prepared statement to a portal.
    ///
    /// This command is part of the extended query flow.
    Bind {
        /// The destination portal. An empty string selects the unnamed
        /// portal. The portal can later be executed with the `Execute` command.
        portal_name: String,
        /// The source prepared statement. An empty string selects the unnamed
        /// prepared statement.
        statement_name: String,
        /// The formats used to encode the parameters.
        param_formats: Vec<PgFormat>,
        /// The value of each parameter.
        raw_params: Vec<Option<Vec<u8>>>,
        /// The desired formats for the columns in the result set.
        result_formats: Vec<PgFormat>,
    },

    /// Execute a bound portal.
    ///
    /// This command is part of the extended query flow.
    Execute {
        /// The name of the portal to execute.
        portal_name: String,
        /// The maximum number of rows to return before suspending.
        ///
        /// 0 or negative means infinite.
        max_rows: i32,
    },

    /// Flush any pending output.
    ///
    /// This command is part of the extended query flow.
    Flush,

    /// Finish an extended query.
    ///
    /// This command is part of the extended query flow.
    Sync,

    /// Close the named statement.
    ///
    /// This command is part of the extended query flow.
    CloseStatement {
        /// The name of the prepared statement to close.
        name: String,
    },

    /// Close the named portal.
    ///
    /// This command is part of the extended query flow.
    ClosePortal {
        /// The name of the portal to close.
        name: String,
    },

    /// Terminate a connection.
    Terminate,
}

impl FrontendMessage {
    /// decodes buffer data to a frontend message
    pub fn decode(tag: u8, buffer: &[u8]) -> Result<Self> {
        log::trace!("Receives frontend tag = {:?}, buffer = {:?}", char::from(tag), buffer);

        let cursor = Cursor::from(buffer);
        match tag {
            // Simple query flow.
            QUERY => decode_query(cursor),

            // Extended query flow.
            BIND => decode_bind(cursor),
            CLOSE => decode_close(cursor),
            DESCRIBE => decode_describe(cursor),
            EXECUTE => decode_execute(cursor),
            FLUSH => decode_flush(cursor),
            PARSE => decode_parse(cursor),
            SYNC => decode_sync(cursor),

            TERMINATE => decode_terminate(cursor),

            // Invalid.
            _ => {
                log::error!("unsupported frontend message tag {}", tag);
                Err(Error::UnsupportedFrontendMessage)
            }
        }
    }
}

/// Backend PostgreSQL Wire Protocol messages
/// see [Protocol Flow](https://www.postgresql.org/docs/current/protocol-flow.html)
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum BackendMessage {
    /// A warning message has been issued. The frontend should display the message
    /// but continue listening for ReadyForQuery or ErrorResponse.
    NoticeResponse,
    /// The frontend must now send a PasswordMessage containing the password in
    /// clear-text form. If this is the correct password, the server responds
    /// with an AuthenticationOk, otherwise it responds with an ErrorResponse.
    AuthenticationCleartextPassword,
    /// The frontend must now send a PasswordMessage containing the password
    /// (with user name) encrypted via MD5, then encrypted again using the 4-byte
    /// random salt specified in the AuthenticationMD5Password message. If this
    /// is the correct password, the server responds with an AuthenticationOk,
    /// otherwise it responds with an ErrorResponse. The actual PasswordMessage
    /// can be computed in SQL as concat('md5', md5(concat(md5(concat(password,
    /// username)), random-salt))). (Keep in mind the md5() function returns its
    /// result as a hex string.)
    AuthenticationMD5Password,
    /// The authentication exchange is successfully completed.
    AuthenticationOk,
    /// Identifies as cancellation key data. The frontend must save these values
    /// if it wishes to be able to issue CancelRequest messages later.
    BackendKeyData(ConnId, ConnSecretKey),
    /// Start-up is completed. The frontend can now issue commands.
    ReadyForQuery,
    /// One of the set of rows returned by a SELECT, FETCH, etc query.
    DataRow(Vec<String>),
    /// Indicates that rows are about to be returned in response to a SELECT, FETCH,
    /// etc query. The contents of this message describe the column layout of
    /// the rows. This will be followed by a DataRow message for each row being
    /// returned to the frontend.
    RowDescription(Vec<ColumnMetadata>),
    /// An SQL command completed normally.
    CommandComplete(String),
    /// An empty query string was recognized.
    EmptyQueryResponse,
    /// An error has occurred. Contains (`Severity`, `Error Code`, `Error Message`)
    /// all of them are optional
    ErrorResponse(Option<&'static str>, Option<&'static str>, Option<String>),
    /// This message informs the frontend about the current (initial) setting of
    /// backend parameters, such as client_encoding or DateStyle
    ///
    /// see https://www.postgresql.org/docs/12/protocol-flow.html#PROTOCOL-ASYNC
    /// 3rd and 4th paragraph
    ParameterStatus(String, String),
    /// Indicates that parameters are needed by a prepared statement.
    ParameterDescription(Vec<PgType>),
    /// Indicates that the statement will not return rows.
    NoData,
    /// This message informs the frontend about the previous `Parse` frontend
    /// message is successful.
    ParseComplete,
    /// This message informs the frontend about the previous `Bind` frontend
    /// message is successful.
    BindComplete,
    /// This message informs the frontend about the previous `Close` frontend
    /// message is successful.
    CloseComplete,
}

impl BackendMessage {
    /// returns binary representation of a backend message
    pub fn as_vec(&self) -> Vec<u8> {
        match self {
            BackendMessage::NoticeResponse => vec![NOTICE_RESPONSE],
            BackendMessage::AuthenticationCleartextPassword => vec![AUTHENTICATION, 0, 0, 0, 8, 0, 0, 0, 3],
            BackendMessage::AuthenticationMD5Password => vec![AUTHENTICATION, 0, 0, 0, 12, 0, 0, 0, 5, 1, 1, 1, 1],
            BackendMessage::AuthenticationOk => vec![AUTHENTICATION, 0, 0, 0, 8, 0, 0, 0, 0],
            BackendMessage::BackendKeyData(conn_id, secret_key) => {
                let mut buff = vec![BACKEND_KEY_DATA, 0, 0, 0, 12];
                buff.extend_from_slice(&conn_id.to_be_bytes());
                buff.extend_from_slice(&secret_key.to_be_bytes());
                buff
            }
            BackendMessage::ReadyForQuery => vec![READY_FOR_QUERY, 0, 0, 0, 5, EMPTY_QUERY_RESPONSE],
            BackendMessage::DataRow(row) => {
                let mut row_buff = Vec::new();
                for field in row.iter() {
                    row_buff.extend_from_slice(&(field.len() as i32).to_be_bytes());
                    row_buff.extend_from_slice(field.as_str().as_bytes());
                }
                let mut len_buff = Vec::new();
                len_buff.extend_from_slice(&[DATA_ROW]);
                len_buff.extend_from_slice(&(6 + row_buff.len() as i32).to_be_bytes());
                len_buff.extend_from_slice(&(row.len() as i16).to_be_bytes());
                len_buff.extend_from_slice(&row_buff);
                len_buff
            }
            BackendMessage::RowDescription(description) => {
                let mut buff = Vec::new();
                for field in description.iter() {
                    buff.extend_from_slice(field.name.as_str().as_bytes());
                    buff.extend_from_slice(&[0]); // end of c string
                    buff.extend_from_slice(&(0i32).to_be_bytes()); // table id
                    buff.extend_from_slice(&(0i16).to_be_bytes()); // column id
                    buff.extend_from_slice(&field.type_id.to_be_bytes());
                    buff.extend_from_slice(&field.type_size.to_be_bytes());
                    buff.extend_from_slice(&(-1i32).to_be_bytes()); // type modifier
                    buff.extend_from_slice(&0i16.to_be_bytes());
                }
                let mut len_buff = Vec::new();
                len_buff.extend_from_slice(&[ROW_DESCRIPTION]);
                len_buff.extend_from_slice(&(6 + buff.len() as i32).to_be_bytes());
                len_buff.extend_from_slice(&(description.len() as i16).to_be_bytes());
                len_buff.extend_from_slice(&buff);
                len_buff
            }
            BackendMessage::CommandComplete(command) => {
                let mut command_buff = Vec::new();
                command_buff.extend_from_slice(&[COMMAND_COMPLETE]);
                command_buff.extend_from_slice(&(4 + command.len() as i32 + 1).to_be_bytes());
                command_buff.extend_from_slice(command.as_bytes());
                command_buff.extend_from_slice(&[0]);
                command_buff
            }
            BackendMessage::EmptyQueryResponse => vec![EMPTY_QUERY_RESPONSE, 0, 0, 0, 4],
            BackendMessage::ErrorResponse(severity, code, message) => {
                let mut error_response_buff = Vec::new();
                error_response_buff.extend_from_slice(&[ERROR_RESPONSE]);
                let mut message_buff = Vec::new();
                if let Some(severity) = severity.as_ref() {
                    message_buff.extend_from_slice(&[SEVERITY]);
                    message_buff.extend_from_slice(severity.as_bytes());
                    message_buff.extend_from_slice(&[0]);
                }
                if let Some(code) = code.as_ref() {
                    message_buff.extend_from_slice(&[CODE]);
                    message_buff.extend_from_slice(code.as_bytes());
                    message_buff.extend_from_slice(&[0]);
                }
                if let Some(message) = message.as_ref() {
                    message_buff.extend_from_slice(&[MESSAGE]);
                    message_buff.extend_from_slice(message.as_bytes());
                    message_buff.extend_from_slice(&[0]);
                }
                error_response_buff.extend_from_slice(&(message_buff.len() as i32 + 4 + 1).to_be_bytes());
                error_response_buff.extend_from_slice(message_buff.as_ref());
                error_response_buff.extend_from_slice(&[0]);
                error_response_buff.to_vec()
            }
            BackendMessage::ParameterStatus(name, value) => {
                let mut parameter_status_buff = Vec::new();
                parameter_status_buff.extend_from_slice(&[PARAMETER_STATUS]);
                let mut parameters = Vec::new();
                parameters.extend_from_slice(name.as_bytes());
                parameters.extend_from_slice(&[0]);
                parameters.extend_from_slice(value.as_bytes());
                parameters.extend_from_slice(&[0]);
                parameter_status_buff.extend_from_slice(&(4 + parameters.len() as u32).to_be_bytes());
                parameter_status_buff.extend_from_slice(parameters.as_ref());
                parameter_status_buff
            }
            BackendMessage::ParameterDescription(pg_types) => {
                let mut type_id_buff = Vec::new();
                for pg_type in pg_types.iter() {
                    type_id_buff.extend_from_slice(&pg_type.type_oid().to_be_bytes());
                }
                let mut buff = Vec::new();
                buff.extend_from_slice(&[PARAMETER_DESCRIPTION]);
                buff.extend_from_slice(&(6 + type_id_buff.len() as i32).to_be_bytes());
                buff.extend_from_slice(&(pg_types.len() as i16).to_be_bytes());
                buff.extend_from_slice(&type_id_buff);
                buff
            }
            BackendMessage::NoData => vec![NO_DATA, 0, 0, 0, 4],
            BackendMessage::ParseComplete => vec![PARSE_COMPLETE, 0, 0, 0, 4],
            BackendMessage::BindComplete => vec![BIND_COMPLETE, 0, 0, 0, 4],
            BackendMessage::CloseComplete => vec![CLOSE_COMPLETE, 0, 0, 0, 4],
        }
    }
}

/// Struct description of metadata that describes how client should interpret
/// outgoing selected data
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnMetadata {
    /// name of the column that was specified in query
    pub name: String,
    /// PostgreSQL data type id
    pub type_id: u32,
    /// PostgreSQL data type size
    pub type_size: i16,
}

impl ColumnMetadata {
    /// Creates new column metadata
    pub fn new<S: ToString>(name: S, pg_type: PgType) -> ColumnMetadata {
        Self {
            name: name.to_string(),
            type_id: pg_type.type_oid(),
            type_size: pg_type.type_len(),
        }
    }
}

fn decode_bind(mut cursor: Cursor) -> Result<FrontendMessage> {
    let portal_name = cursor.read_cstr()?.to_owned();
    let statement_name = cursor.read_cstr()?.to_owned();

    let mut param_formats = vec![];
    for _ in 0..cursor.read_i16()? {
        match PgFormat::try_from(cursor.read_i16()?) {
            Ok(format) => param_formats.push(format),
            Err(UnrecognizedFormat(code)) => return Err(Error::InvalidInput(format!("unknown format code: {}", code))),
        }
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
        match PgFormat::try_from(cursor.read_i16()?) {
            Ok(format) => result_formats.push(format),
            Err(UnrecognizedFormat(code)) => return Err(Error::InvalidInput(format!("unknown format code: {}", code))),
        }
    }

    Ok(FrontendMessage::Bind {
        portal_name,
        statement_name,
        param_formats,
        raw_params,
        result_formats,
    })
}

fn decode_close(mut cursor: Cursor) -> Result<FrontendMessage> {
    let first_char = cursor.read_byte()?;
    let name = cursor.read_cstr()?.to_owned();
    match first_char {
        b'P' => Ok(FrontendMessage::ClosePortal { name }),
        b'S' => Ok(FrontendMessage::CloseStatement { name }),
        other => Err(Error::InvalidInput(format!(
            "invalid type byte in Close frontend message: {:?}",
            std::char::from_u32(other as u32).unwrap(),
        ))),
    }
}

fn decode_describe(mut cursor: Cursor) -> Result<FrontendMessage> {
    let first_char = cursor.read_byte()?;
    let name = cursor.read_cstr()?.to_owned();
    match first_char {
        b'P' => Ok(FrontendMessage::DescribePortal { name }),
        b'S' => Ok(FrontendMessage::DescribeStatement { name }),
        other => Err(Error::InvalidInput(format!(
            "invalid type byte in Describe frontend message: {:?}",
            char::from(other),
        ))),
    }
}

fn decode_execute(mut cursor: Cursor) -> Result<FrontendMessage> {
    let portal_name = cursor.read_cstr()?.to_owned();
    let max_rows = cursor.read_i32()?;
    Ok(FrontendMessage::Execute { portal_name, max_rows })
}

fn decode_flush(_cursor: Cursor) -> Result<FrontendMessage> {
    Ok(FrontendMessage::Flush)
}

fn decode_parse(mut cursor: Cursor) -> Result<FrontendMessage> {
    let statement_name = cursor.read_cstr()?.to_owned();
    let sql = cursor.read_cstr()?.to_owned();

    let mut param_types = vec![];
    for _ in 0..cursor.read_i16()? {
        let oid = PgType::from_oid(cursor.read_u32()?)?;
        log::trace!("OID {:?}", oid);
        param_types.push(oid);
    }

    Ok(FrontendMessage::Parse {
        statement_name,
        sql,
        param_types,
    })
}

fn decode_sync(_cursor: Cursor) -> Result<FrontendMessage> {
    Ok(FrontendMessage::Sync)
}

fn decode_query(mut cursor: Cursor) -> Result<FrontendMessage> {
    let sql = cursor.read_cstr()?.to_owned();
    Ok(FrontendMessage::Query { sql })
}

fn decode_terminate(_cursor: Cursor) -> Result<FrontendMessage> {
    Ok(FrontendMessage::Terminate)
}

#[cfg(test)]
mod decoding_frontend_messages {
    use super::*;

    #[test]
    fn query() {
        let buffer = [
            99, 114, 101, 97, 116, 101, 32, 115, 99, 104, 101, 109, 97, 32, 115, 99, 104, 101, 109, 97, 95, 110, 97,
            109, 101, 59, 0,
        ];
        let message = FrontendMessage::decode(b'Q', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::Query {
                sql: "create schema schema_name;".to_owned()
            })
        );
    }

    #[test]
    fn bind() {
        let buffer = [
            112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0, 115, 116, 97, 116, 101, 109, 101, 110, 116, 95, 110,
            97, 109, 101, 0, 0, 2, 0, 1, 0, 1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0,
        ];
        let message = FrontendMessage::decode(b'B', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::Bind {
                portal_name: "portal_name".to_owned(),
                statement_name: "statement_name".to_owned(),
                param_formats: vec![PgFormat::Binary, PgFormat::Binary],
                raw_params: vec![Some(vec![0, 0, 0, 1]), Some(vec![0, 0, 0, 2])],
                result_formats: vec![],
            })
        );
    }

    #[test]
    fn close_portal() {
        let buffer = [80, 112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0];
        let message = FrontendMessage::decode(b'C', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::ClosePortal {
                name: "portal_name".to_owned(),
            })
        );
    }

    #[test]
    fn close_statement() {
        let buffer = [83, 115, 116, 97, 116, 101, 109, 101, 110, 116, 95, 110, 97, 109, 101, 0];
        let message = FrontendMessage::decode(b'C', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::CloseStatement {
                name: "statement_name".to_owned(),
            })
        );
    }

    #[test]
    fn describe_portal() {
        let buffer = [80, 112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0];
        let message = FrontendMessage::decode(b'D', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::DescribePortal {
                name: "portal_name".to_owned()
            })
        );
    }

    #[test]
    fn describe_statement() {
        let buffer = [83, 115, 116, 97, 116, 101, 109, 101, 110, 116, 95, 110, 97, 109, 101, 0];
        let message = FrontendMessage::decode(b'D', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::DescribeStatement {
                name: "statement_name".to_owned()
            })
        );
    }

    #[test]
    fn execute() {
        let buffer = [112, 111, 114, 116, 97, 108, 95, 110, 97, 109, 101, 0, 0, 0, 0, 0];
        let message = FrontendMessage::decode(b'E', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::Execute {
                portal_name: "portal_name".to_owned(),
                max_rows: 0,
            })
        );
    }

    #[test]
    fn flush() {
        let message = FrontendMessage::decode(b'H', &[]);
        assert_eq!(message, Ok(FrontendMessage::Flush));
    }

    #[test]
    fn parse() {
        let buffer = [
            0, 115, 101, 108, 101, 99, 116, 32, 42, 32, 102, 114, 111, 109, 32, 115, 99, 104, 101, 109, 97, 95, 110,
            97, 109, 101, 46, 116, 97, 98, 108, 101, 95, 110, 97, 109, 101, 32, 119, 104, 101, 114, 101, 32, 115, 105,
            95, 99, 111, 108, 117, 109, 110, 32, 61, 32, 36, 49, 59, 0, 0, 1, 0, 0, 0, 23,
        ];
        let message = FrontendMessage::decode(b'P', &buffer);
        assert_eq!(
            message,
            Ok(FrontendMessage::Parse {
                statement_name: "".to_owned(),
                sql: "select * from schema_name.table_name where si_column = $1;".to_owned(),
                param_types: vec![Some(PgType::Integer)],
            })
        );
    }

    #[test]
    fn sync() {
        let message = FrontendMessage::decode(b'S', &[]);
        assert_eq!(message, Ok(FrontendMessage::Sync));
    }

    #[test]
    fn terminate() {
        let message = FrontendMessage::decode(b'X', &[]);
        assert_eq!(message, Ok(FrontendMessage::Terminate));
    }
}

#[cfg(test)]
mod serializing_backend_messages {
    use super::*;

    #[test]
    fn notice() {
        assert_eq!(BackendMessage::NoticeResponse.as_vec(), vec![NOTICE_RESPONSE]);
    }

    #[test]
    fn authentication_cleartext_password() {
        assert_eq!(
            BackendMessage::AuthenticationCleartextPassword.as_vec(),
            vec![AUTHENTICATION, 0, 0, 0, 8, 0, 0, 0, 3]
        )
    }

    #[test]
    fn authentication_md5_password() {
        assert_eq!(
            BackendMessage::AuthenticationMD5Password.as_vec(),
            vec![AUTHENTICATION, 0, 0, 0, 12, 0, 0, 0, 5, 1, 1, 1, 1]
        )
    }

    #[test]
    fn authentication_ok() {
        assert_eq!(
            BackendMessage::AuthenticationOk.as_vec(),
            vec![AUTHENTICATION, 0, 0, 0, 8, 0, 0, 0, 0]
        )
    }

    #[test]
    fn backend_key_data() {
        assert_eq!(
            BackendMessage::BackendKeyData(1, 2).as_vec(),
            vec![BACKEND_KEY_DATA, 0, 0, 0, 12, 0, 0, 0, 1, 0, 0, 0, 2]
        )
    }

    #[test]
    fn parameter_status() {
        assert_eq!(
            BackendMessage::ParameterStatus("client_encoding".to_owned(), "UTF8".to_owned()).as_vec(),
            vec![
                PARAMETER_STATUS,
                0,
                0,
                0,
                25,
                99,
                108,
                105,
                101,
                110,
                116,
                95,
                101,
                110,
                99,
                111,
                100,
                105,
                110,
                103,
                0,
                85,
                84,
                70,
                56,
                0
            ]
        )
    }

    #[test]
    fn ready_for_query() {
        assert_eq!(
            BackendMessage::ReadyForQuery.as_vec(),
            vec![READY_FOR_QUERY, 0, 0, 0, 5, EMPTY_QUERY_RESPONSE]
        )
    }

    #[test]
    fn data_row() {
        assert_eq!(
            BackendMessage::DataRow(vec!["1".to_owned(), "2".to_owned(), "3".to_owned()]).as_vec(),
            vec![DATA_ROW, 0, 0, 0, 21, 0, 3, 0, 0, 0, 1, 49, 0, 0, 0, 1, 50, 0, 0, 0, 1, 51]
        )
    }

    #[test]
    fn row_description() {
        assert_eq!(
            BackendMessage::RowDescription(vec![ColumnMetadata::new("c1".to_owned(), PgType::Integer)]).as_vec(),
            vec![
                ROW_DESCRIPTION,
                0,
                0,
                0,
                27,
                0,
                1,
                99,
                49,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                23,
                0,
                4,
                255,
                255,
                255,
                255,
                0,
                0
            ]
        );
    }

    #[test]
    fn command_complete() {
        assert_eq!(
            BackendMessage::CommandComplete("SELECT".to_owned()).as_vec(),
            vec![COMMAND_COMPLETE, 0, 0, 0, 11, 83, 69, 76, 69, 67, 84, 0]
        )
    }

    #[test]
    fn empty_response() {
        assert_eq!(
            BackendMessage::EmptyQueryResponse.as_vec(),
            vec![EMPTY_QUERY_RESPONSE, 0, 0, 0, 4]
        )
    }

    #[test]
    fn error_response() {
        assert_eq!(
            BackendMessage::ErrorResponse(None, None, None).as_vec(),
            vec![ERROR_RESPONSE, 0, 0, 0, 5, 0]
        )
    }

    #[test]
    fn parameter_description() {
        assert_eq!(
            BackendMessage::ParameterDescription(vec![PgType::Integer]).as_vec(),
            vec![PARAMETER_DESCRIPTION, 0, 0, 0, 10, 0, 1, 0, 0, 0, 23]
        )
    }

    #[test]
    fn no_data() {
        assert_eq!(BackendMessage::NoData.as_vec(), vec![NO_DATA, 0, 0, 0, 4])
    }

    #[test]
    fn parse_complete() {
        assert_eq!(BackendMessage::ParseComplete.as_vec(), vec![PARSE_COMPLETE, 0, 0, 0, 4])
    }

    #[test]
    fn bind_complete() {
        assert_eq!(BackendMessage::BindComplete.as_vec(), vec![BIND_COMPLETE, 0, 0, 0, 4])
    }

    #[test]
    fn close_complete() {
        assert_eq!(BackendMessage::CloseComplete.as_vec(), vec![CLOSE_COMPLETE, 0, 0, 0, 4])
    }
}
