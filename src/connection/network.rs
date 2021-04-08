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

cfg_if::cfg_if! {
    if #[cfg(feature = "async_net")] {
        pub use async_net::*;
    } else if #[cfg(feature = "tokio_net")] {
        pub use tokio_net::*;
    } else {
        pub use mock::*;
    }
}

#[cfg(feature = "mock_network")]
pub (crate) mod mock;
#[cfg(feature = "async_net")]
mod async_net;
#[cfg(feature = "tokio_net")]
mod tokio_net;
