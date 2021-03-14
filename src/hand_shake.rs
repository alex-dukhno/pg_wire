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
    errors::{HandShakeError, HandShakeErrorKind},
    request_codes::{Code, CANCEL_REQUEST_CODE, SSL_REQUEST_CODE, VERSION_1_CODE, VERSION_2_CODE, VERSION_3_CODE},
    ConnId, ConnSecretKey,
};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum State {
    MessageLen,
    ParseSetup,
}

/// Encapsulate protocol hand shake process
///
/// # Examples
///
/// ```ignore
/// use pg_wire::{HandShakeProcess, HandShakeStatus, HandShakeRequest};
///
/// let mut stream = accept_tcp_connection();
/// let mut process = HandShakeProcess::start();
/// let mut buffer: Option<Vec<u8>> = None;
/// loop {
///     match process.next_stage(buffer.as_deref()) {
///         Ok(HandShakeStatus::RequestingBytes(len)) => {
///             let mut buf = vec![b'0'; len];
///             buffer = Some(stream.read(&mut buf)?);
///         }
///         Ok(HandShakeStatus::UpdatingToSecureWithReadingBytes(len)) => {
///             stream.write_all(&[b'S']); // accepting tls connection from client
///             stream = tls_stream(stream);
///             let mut buf = vec![b'0'; len];
///             buffer = Some(stream.read_exact(&mut buf)?);
///         }
///         Ok(HandShakeStatus::Cancel(conn_id, secret_key)) => {
///             handle_request_cancellation(conn_id, secret_key);
///             break;
///         }
///         Ok(HandShakeStatus::Done(props)) => {
///             handle_authentication_and_other_stuff();
///             break;
///         }
///         Err(protocol_error) => {
///             handle_protocol_error(protocol_error);
///             break;
///         }
///     }
/// }
/// ```
pub struct Process {
    state: Option<State>,
}

impl Process {
    /// Creates new process to make client <-> server hand shake
    pub fn start() -> Process {
        Process { state: None }
    }

    /// Proceed to the next stage of client <-> server hand shake
    pub fn next_stage<'e>(&mut self, payload: Option<&'e [u8]>) -> Result<Status, HandShakeError<'e>> {
        match self.state.take().and_then(|state| payload.map(|buf| (state, buf))) {
            None => {
                self.state = Some(State::MessageLen);
                Ok(Status::RequestingBytes(4))
            }
            Some((state, bytes)) => match state {
                State::MessageLen => {
                    let mut buffer = Cursor::from(bytes);
                    let len = buffer.read_i32()?;
                    self.state = Some(State::ParseSetup);
                    Ok(Status::RequestingBytes((len - 4) as usize))
                }
                State::ParseSetup => {
                    let mut buffer = Cursor::from(bytes);
                    let code = Code(buffer.read_i32()?);
                    match code {
                        VERSION_1_CODE | VERSION_2_CODE => Err(HandShakeError::from(
                            HandShakeErrorKind::UnsupportedProtocolVersion(code),
                        )),
                        VERSION_3_CODE => {
                            let mut props = vec![];
                            loop {
                                let key = buffer.read_cstr()?.to_owned();
                                if key.is_empty() {
                                    break;
                                }
                                let value = buffer.read_cstr()?.to_owned();
                                props.push((key, value));
                            }
                            Ok(Status::Done(props))
                        }
                        CANCEL_REQUEST_CODE => {
                            let conn_id = buffer.read_i32()?;
                            let secret_key = buffer.read_i32()?;
                            Ok(Status::Cancel(conn_id, secret_key))
                        }
                        SSL_REQUEST_CODE => {
                            self.state = Some(State::MessageLen);
                            Ok(Status::UpdatingToSecureWithReadingBytes(4))
                        }
                        otherwise => Err(HandShakeError::from(HandShakeErrorKind::UnsupportedClientRequest(
                            otherwise,
                        ))),
                    }
                }
            },
        }
    }
}

/// Represents status of the [HandShakeProcess](Process) stages
#[derive(Debug, PartialEq)]
pub enum Status {
    /// Hand shake process requesting additional data to proceed further
    RequestingBytes(usize),
    /// Hand shake process requesting update to SSL and additional data to proceed further
    UpdatingToSecureWithReadingBytes(usize),
    /// Hand shake is finished. Contains client runtime settings, e.g. database, username
    Done(Vec<(String, String)>),
    /// Hand shake is for canceling request that is executed on `ConnId`
    Cancel(ConnId, ConnSecretKey),
}

/// Hand shake request to a server process
#[derive(Debug, PartialEq)]
pub enum Request {
    /// Server should provide `Process` with buffer of request size
    Buffer(usize),
    /// Server should use SSL protocol over current connection stream
    UpgradeToSsl,
}

#[cfg(test)]
mod perform_hand_shake_loop {
    use super::*;
    use crate::request_codes::{CANCEL_REQUEST_CODE, SSL_REQUEST_CODE, VERSION_3_CODE};

    #[test]
    fn init_hand_shake_process() {
        let mut process = Process::start();
        assert_eq!(process.next_stage(None), Ok(Status::RequestingBytes(4)));
    }

    #[test]
    fn read_setup_message_length() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        assert_eq!(
            process.next_stage(Some(&[0, 0, 0, 33])),
            Ok(Status::RequestingBytes(29))
        );
    }

    #[test]
    fn non_secure_connection_hand_shake() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        process
            .next_stage(Some(&[0, 0, 0, 33]))
            .expect("proceed to the next stage");

        let mut payload = vec![];
        payload.extend_from_slice(&Vec::from(VERSION_3_CODE));
        payload.extend_from_slice(b"key1\0");
        payload.extend_from_slice(b"value1\0");
        payload.extend_from_slice(b"key2\0");
        payload.extend_from_slice(b"value2\0");
        payload.extend_from_slice(&[0]);

        assert_eq!(
            process.next_stage(Some(&payload)),
            Ok(Status::Done(vec![
                ("key1".to_owned(), "value1".to_owned()),
                ("key2".to_owned(), "value2".to_owned())
            ]))
        );
    }

    #[test]
    fn not_supported_version() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        process
            .next_stage(Some(&[0, 0, 0, 33]))
            .expect("proceed to the next stage");

        let mut payload = vec![];
        payload.extend_from_slice(&Vec::from(VERSION_2_CODE));
        payload.extend_from_slice(b"key1\0");
        payload.extend_from_slice(b"value1\0");
        payload.extend_from_slice(b"key2\0");
        payload.extend_from_slice(b"value2\0");
        payload.extend_from_slice(&[0]);

        assert_eq!(
            process.next_stage(Some(&payload)),
            Err(HandShakeError::from(HandShakeErrorKind::UnsupportedProtocolVersion(VERSION_2_CODE)))
        );
    }

    #[test]
    fn not_supported_client_request() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        process
            .next_stage(Some(&[0, 0, 0, 33]))
            .expect("proceed to the next stage");

        let mut payload = vec![];
        payload.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        payload.extend_from_slice(b"key1\0");
        payload.extend_from_slice(b"value1\0");
        payload.extend_from_slice(b"key2\0");
        payload.extend_from_slice(b"value2\0");
        payload.extend_from_slice(&[0]);

        assert_eq!(
            process.next_stage(Some(&payload)),
            Err(HandShakeError::from(HandShakeErrorKind::UnsupportedClientRequest(Code(0x11_22_33_44))))
        );
    }

    #[test]
    fn ssl_secure_connection_hand_shake() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        process
            .next_stage(Some(&[0, 0, 0, 8]))
            .expect("proceed to the next stage");

        assert_eq!(
            process.next_stage(Some(&Vec::from(SSL_REQUEST_CODE))),
            Ok(Status::UpdatingToSecureWithReadingBytes(4))
        );

        process
            .next_stage(Some(&[0, 0, 0, 33]))
            .expect("proceed to the next stage");

        let mut payload = vec![];
        payload.extend_from_slice(&Vec::from(VERSION_3_CODE));
        payload.extend_from_slice(b"key1\0");
        payload.extend_from_slice(b"value1\0");
        payload.extend_from_slice(b"key2\0");
        payload.extend_from_slice(b"value2\0");
        payload.extend_from_slice(&[0]);

        assert_eq!(
            process.next_stage(Some(&payload)),
            Ok(Status::Done(vec![
                ("key1".to_owned(), "value1".to_owned()),
                ("key2".to_owned(), "value2".to_owned())
            ]))
        );
    }

    #[test]
    fn cancel_query_request() {
        let conn_id: ConnId = 1;
        let secret_key: ConnSecretKey = 2;

        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        process
            .next_stage(Some(&[0, 0, 0, 16]))
            .expect("proceed to the next stage");

        let mut payload = vec![];
        payload.extend_from_slice(&Vec::from(CANCEL_REQUEST_CODE));
        payload.extend_from_slice(&conn_id.to_be_bytes());
        payload.extend_from_slice(&secret_key.to_be_bytes());

        assert_eq!(
            process.next_stage(Some(&payload)),
            Ok(Status::Cancel(conn_id, secret_key))
        );
    }
}
