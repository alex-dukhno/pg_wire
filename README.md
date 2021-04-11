# PostgreSQL Wire Protocol

![ci](https://github.com/alex-dukhno/pg_wire/workflows/ci/badge.svg)
[![Coverage Status](https://coveralls.io/repos/github/alex-dukhno/pg_wire/badge.svg?branch=main)](https://coveralls.io/github/alex-dukhno/pg_wire?branch=main)
[![Cargo](https://img.shields.io/crates/v/pg_wire.svg)](https://crates.io/crates/pg_wire)
[![Documentation](https://docs.rs/pg_wire/badge.svg)](https://docs.rs/pg_wire)
<a href="https://discord.gg/PUcTcfU"><img src="https://img.shields.io/discord/509773073294295082.svg?logo=discord"></a>

The crate supports:
 * [tokio](https://tokio.rs) runtime,
 * [smol](https://github.com/smol-rs/smol) runtimes. Actually, it depends on [async-io](https://github.com/smol-rs/async-io) on which smol is based.

Add this to you `Cargo.toml` if you want to use it with `tokio` runtime
```toml
pg_wire = { version = "0.9.0", features = ["tokio_net"] }
```

Add this to you `Cargo.toml` if you want to enable `smol` runtime support
```toml
pg_wire = { version = "0.9.0", features = ["async_io"] }
```

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

### Secured smol runtime

Open your terminal and run the following command:
```shell
cargo run --example secured_smol_server --features async_io
```
Open another terminal window and run:
```shell
psql -h 127.0.0.1 -U postgres -p 5432 -W
```
Enter any password
After that in the console you should see something like
```shell
psql (13.2, server 12.4)
SSL connection (protocol: TLSv1.2, cipher: ECDHE-RSA-AES256-GCM-SHA384, bits: 256, compression: off)
Type "help" for help.

postgres=> select 3;
 col1
------
    1
(1 row)

postgres=>
```

The server always handles `select 1` SQL query
