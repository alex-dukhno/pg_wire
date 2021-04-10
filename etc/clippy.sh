#!/bin/bash

cd "$(dirname "$0")"/..
set -ex

cargo clippy --all --features async_io \
      && cargo clippy --all --features tokio_net \
      && cargo clippy --all --features mock_net
