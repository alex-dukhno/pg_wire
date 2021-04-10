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

use std::path::PathBuf;

fn main() {
    #[cfg(not(feature = "async_io"))]
    println!("execute `cargo run --example secured_smol_server --features async_io` to run this example");
    #[cfg(feature = "async_io")]
        smol::block_on(async {
        use async_mutex::Mutex as AsyncMutex;
        use futures_lite::{AsyncReadExt, AsyncWriteExt};
        use pg_wire::{
            ClientRequest, CommandMessage, ConnSupervisor, Connection, PgWireListener, ProtocolConfiguration, Sender
        };
        use pg_wire_payload::{BackendMessage, ColumnMetadata, PgType};
        use smol::Async;
        use std::{net::TcpListener, sync::Arc};

        let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 5432)).expect("OK");
        println!("server started");

        let config = ProtocolConfiguration::with_ssl(PathBuf::from("./etc/identity.pfx"), "password".to_owned());
        let conn_supervisor = ConnSupervisor::new(0, 10);
        let pg_wire_listener = PgWireListener::new(listener, config, conn_supervisor);

        loop {
            match pg_wire_listener.accept().await {
                Err(io_error) => eprintln!("IO error {:?}", io_error),
                Ok(Err(protocol_error)) => eprintln!("protocol error {}", protocol_error),
                Ok(Ok(ClientRequest::Connect((mut channel, props, conn_supervisor, address)))) => {
                    channel
                        .write_all(BackendMessage::AuthenticationCleartextPassword.as_vec().as_slice())
                        .await
                        .expect("to ask for password in clear text format");
                    channel.flush().await.expect("to flush the buffer");

                    //TODO: use message decoder for Auth messages
                    let mut tag_buffer = [0u8; 1];
                    let _tag = channel.read_exact(&mut tag_buffer).await.map(|_| tag_buffer[0]);
                    let mut len_buffer = [0u8; 4];
                    let len = channel
                        .read_exact(&mut len_buffer)
                        .await
                        .map(|_| u32::from_be_bytes(len_buffer) as usize)
                        .expect("to read message length");
                    let len = len - 4;
                    let mut message_buffer = Vec::with_capacity(len);
                    message_buffer.resize(len, b'0');
                    let _message = channel
                        .read_exact(&mut message_buffer)
                        .await
                        .map(|_| message_buffer)
                        .expect("to read message body");

                    // we are ok with any password that user sent
                    channel
                        .write_all(BackendMessage::AuthenticationOk.as_vec().as_slice())
                        .await
                        .expect("Auth Ok");

                    // pretend to be a PostgreSQL version 12.4
                    channel
                        .write_all(
                            BackendMessage::ParameterStatus("server_version".to_owned(), "12.4".to_owned())
                                .as_vec()
                                .as_slice(),
                        )
                        .await
                        .expect("send server version");

                    let (conn_id, secret_key) = match conn_supervisor.alloc() {
                        Ok((c, s)) => (c, s),
                        Err(()) => {
                            eprintln!("Cannot allocate connection and its secret key");
                            return;
                        }
                    };

                    // sending connection id and its secret key if client wanted to cancel query
                    channel
                        .write_all(BackendMessage::BackendKeyData(conn_id, secret_key).as_vec().as_slice())
                        .await
                        .expect("to send connection id and secret key");

                    channel
                        .write_all(BackendMessage::ReadyForQuery.as_vec().as_slice())
                        .await
                        .expect("to notify that we ready to handle query");

                    let channel = Arc::new(AsyncMutex::new(channel));
                    let mut connection = Connection::new(conn_id, props, address, channel, conn_supervisor);
                    println!("client connected from {:?}", connection.address());
                    let sender = connection.sender();
                    loop {
                        match connection.receive().await {
                            Err(e) => {
                                eprintln!("Err(e) UNEXPECTED ERROR: {:?}", e);
                                return;
                            }
                            Ok(Err(e)) => {
                                eprintln!("Ok(Err(e)) UNEXPECTED ERROR: {:?}", e);
                                return;
                            }
                            Ok(Ok(command)) => match command {
                                CommandMessage::Query { sql } => {
                                    println!("received query: '{}'", sql);
                                    println!("but anyway we will handle 'select 1'");
                                    sender
                                        .send(BackendMessage::RowDescription(vec![ColumnMetadata::new(
                                            &"col1",
                                            PgType::Integer,
                                        )]))
                                        .expect("Ok");
                                    sender
                                        .send(BackendMessage::DataRow(vec!["1".to_owned()]))
                                        .expect("Ok");
                                    sender
                                        .send(BackendMessage::CommandComplete("SELECT 1".to_owned()))
                                        .expect("Ok");
                                    sender.send(BackendMessage::ReadyForQuery).expect("Ok");
                                }
                                CommandMessage::Terminate => {
                                    println!("close connection");
                                    break;
                                }
                                other => {
                                    println!("{:?} is not supported. Only simple query is supported", other);
                                    sender.send(BackendMessage::NoticeResponse).expect("Ok");
                                    sender.send(BackendMessage::ReadyForQuery).expect("Ok");
                                }
                            },
                        }
                    }
                }
                Ok(Ok(ClientRequest::QueryCancellation(_))) => {
                    println!("Query cancellation is not supported")
                }
            }
        }
    })
}
