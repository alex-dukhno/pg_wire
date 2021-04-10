# PostgreSQL Wire Protocol

![ci](https://github.com/alex-dukhno/pg_wire/workflows/ci/badge.svg)
[![Coverage Status](https://coveralls.io/repos/github/alex-dukhno/pg_wire/badge.svg?branch=main)](https://coveralls.io/github/alex-dukhno/pg_wire?branch=main)
<a href="https://discord.gg/PUcTcfU"><img src="https://img.shields.io/discord/509773073294295082.svg?logo=discord"></a>

The crate supports:
 * [tokio](https://tokio.rs) runtime,
 * [smol](https://github.com/smol-rs/smol) runtimes. Actually, it depends on [async-io](https://github.com/smol-rs/async-io) on which smol is based.



## Examples

### Using smol runtime

Open your terminal and run the following command: 
```shell
cargo run --example smol_server --features async_io
```
Open another terminal window and run:
```shell
psql -h 127.0.0.1 -U postgres -p 5432 -W
```
Enter any password
The server always handles `select 1` SQL query

### Using tokio runtime

Open your terminal and run the following command:
```shell
cargo run --example tokio_server --features tokio_net
```
Open another terminal window and run:
```shell
psql -h 127.0.0.1 -U postgres -p 5432 -W
```
Enter any password
The server always handles `select 1` SQL query
