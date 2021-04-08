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

fn main() {
    #[cfg(not(feature = "tokio_net"))]
    println!("execute `cargo run --example tokio_server --features tokio_net` to run this example");
    #[cfg(feature = "tokio_net")]
    {
        use pg_wire::*;
        use std::{env, error::Error, io};
        use tokio::net::TcpListener;

        let rt = tokio::runtime::Runtime::new().expect("run");

        rt.block_on(async {
            let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:5432".to_string());

            println!("server started");

            let listener = TcpListener::bind(addr).await.expect("bind to address");
            let config = ProtocolConfiguration::none();
            let conn_supervisor = ConnSupervisor::new(0, 10);
            let connection_manager = PgWireListener::new(Network::from(listener), config, conn_supervisor);

            loop {
                match connection_manager.accept().await {
                    Err(io_error) => eprintln!("IO error {:?}", io_error),
                    Ok(Err(protocol_error)) => eprintln!("protocol error {:?}", protocol_error),
                    Ok(Ok(ClientRequest::Connect(mut connection))) => {
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
                                            .send(Ok(BackendMessage::RowDescription(vec![ColumnMetadata::new(
                                                &"col1",
                                                PgType::Integer,
                                            )])))
                                            .expect("Ok");
                                        sender
                                            .send(Ok(BackendMessage::DataRow(vec!["1".to_owned()])))
                                            .expect("Ok");
                                        sender
                                            .send(Ok(BackendMessage::CommandComplete("SELECT 1".to_owned())))
                                            .expect("Ok");
                                        sender.send(Ok(BackendMessage::ReadyForQuery)).expect("Ok");
                                    }
                                    CommandMessage::Terminate => {
                                        println!("close connection");
                                        break;
                                    }
                                    other => {
                                        println!("{:?} is not supported. Only simple query is supported", other);
                                        sender.send(Err(BackendMessage::NoticeResponse)).expect("Ok");
                                        sender.send(Ok(BackendMessage::ReadyForQuery)).expect("Ok");
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
        });
    }
}
