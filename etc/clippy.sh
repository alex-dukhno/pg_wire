#!/bin/bash

cd "$(dirname "$0")"/..
set -ex

cargo clippy --features async_net \
      && cargo clippy --features tokio_net \
      && clippy --features mock_net
