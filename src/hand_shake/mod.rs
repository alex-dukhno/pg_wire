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

use crate::{ConnId, ConnSecretKey, Error, cursor::Cursor};
use crate::{
    request_codes::{Code, CANCEL_REQUEST_CODE, SSL_REQUEST_CODE, VERSION_1_CODE, VERSION_2_CODE, VERSION_3_CODE},
};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum SetupParsed {
    Established(Vec<(String, String)>),
    Cancel(ConnId, ConnSecretKey),
    Secure,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum State {
    MessageLen,
    ParseSetup,
    SetupParsed(SetupParsed),
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
///         Ok(HandShakeStatus::Requesting(HandShakeRequest::Buffer(len))) => {
///             let mut buf = vec![b'0'; len];
///             buffer = Some(stream.read(&mut buf));
///         }
///         Ok(HandShakeStatus::Requesting(HandShakeRequest::UpgradeToSsl)) => {
///             stream.write_all(&[b'S']); // accepting tls connection from client
///             stream = tls_stream(stream);
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
    pub fn next_stage(&mut self, payload: Option<&[u8]>) -> Result<Status, Error> {
        match self.state.take() {
            None => {
                self.state = Some(State::MessageLen);
                Ok(Status::Requesting(Request::Buffer(4)))
            }
            Some(state) => {
                if let Some(bytes) = payload {
                    let mut buffer = Cursor::from(bytes);
                    let (result, new_state) = match state {
                        State::MessageLen => {
                            let len = buffer.read_i32()?;
                            (Status::Requesting(Request::Buffer((len - 4) as usize)), State::ParseSetup)
                        },
                        State::ParseSetup => {
                            let code = Code(buffer.read_i32()?);
                            match code {
                                VERSION_1_CODE | VERSION_2_CODE => return Err(Error::UnsupportedVersion(code)),
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
                                    (Status::Done(props.clone()), State::SetupParsed(SetupParsed::Established(props)))
                                }
                                CANCEL_REQUEST_CODE => {
                                    let conn_id = buffer.read_i32()?;
                                    let secret_key = buffer.read_i32()?;
                                    (Status::Cancel(conn_id, secret_key), State::SetupParsed(SetupParsed::Cancel(conn_id, secret_key)))
                                }
                                SSL_REQUEST_CODE => (Status::Requesting(Request::UpgradeToSsl), State::SetupParsed(SetupParsed::Secure)),
                                otherwise => return Err(Error::UnsupportedRequest(otherwise)),
                            }
                        },
                        _ => return Err(Error::VerificationFailed),
                    };
                    self.state = Some(new_state);
                    Ok(result)
                } else {
                    self.state = match state {
                        State::SetupParsed(SetupParsed::Secure) => Some(State::MessageLen),
                        _ => return Err(Error::VerificationFailed),
                    };
                    Ok(Status::Requesting(Request::Buffer(4)))
                }
            }
        }
    }
}

/// Represents status of the [HandShakeProcess](Process) stages
#[derive(Debug, PartialEq)]
pub enum Status {
    /// Hand shake process requesting additional data or action to proceed further
    Requesting(Request),
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
        assert_eq!(process.next_stage(None), Ok(Status::Requesting(Request::Buffer(4))));
    }

    #[test]
    fn read_setup_message_length() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        assert_eq!(
            process.next_stage(Some(&[0, 0, 0, 33])),
            Ok(Status::Requesting(Request::Buffer(29)))
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
    fn ssl_secure_connection_hand_shake() {
        let mut process = Process::start();

        process.next_stage(None).expect("proceed to the next stage");
        process
            .next_stage(Some(&[0, 0, 0, 8]))
            .expect("proceed to the next stage");

        assert_eq!(
            process.next_stage(Some(&Vec::from(SSL_REQUEST_CODE))),
            Ok(Status::Requesting(Request::UpgradeToSsl))
        );

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
