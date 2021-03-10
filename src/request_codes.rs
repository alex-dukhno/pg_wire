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

use std::fmt::{self, Display, Formatter};

/// Version 1 of the protocol
pub(crate) const VERSION_1_CODE: Code = Code(0x00_01_00_00);
/// Version 2 of the protocol
pub(crate) const VERSION_2_CODE: Code = Code(0x00_02_00_00);
/// Version 3 of the protocol
pub(crate) const VERSION_3_CODE: Code = Code(0x00_03_00_00);
/// Client initiate cancel of a command
pub(crate) const CANCEL_REQUEST_CODE: Code = Code(80_877_102);
/// Client initiate `ssl` connection
pub(crate) const SSL_REQUEST_CODE: Code = Code(80_877_103);
/// Client initiate `gss` encrypted connection
pub(crate) const GSSENC_REQUEST_CODE: Code = Code(80_877_104);

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Code(pub(crate) i32);

impl Display for Code {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            &CANCEL_REQUEST_CODE => write!(f, "Cancel Request"),
            &SSL_REQUEST_CODE => write!(f, "SSL Request"),
            &GSSENC_REQUEST_CODE => write!(f, "GSSENC Request"),
            &VERSION_1_CODE | &VERSION_2_CODE | &VERSION_3_CODE => write!(
                f,
                "Version {}.{} Request",
                (self.0 >> 16) as i16,
                (self.0 & 0x00_00_FF_FF) as i16
            ),
            other => write!(
                f,
                "High bytes 0x{:x?} Low bytes: 0x{:x?}",
                (other.0 >> 16) as i16,
                (other.0 & 0x00_00_FF_FF) as i16
            ),
        }
    }
}

impl From<Code> for Vec<u8> {
    fn from(code: Code) -> Vec<u8> {
        code.0.to_be_bytes().to_vec()
    }
}

#[cfg(test)]
mod code_display_tests {
    use super::*;

    #[test]
    fn version_one_request() {
        assert_eq!(VERSION_1_CODE.to_string(), "Version 1.0 Request");
    }

    #[test]
    fn version_two_request() {
        assert_eq!(VERSION_2_CODE.to_string(), "Version 2.0 Request");
    }

    #[test]
    fn version_three_request() {
        assert_eq!(VERSION_3_CODE.to_string(), "Version 3.0 Request");
    }

    #[test]
    fn cancel_request() {
        assert_eq!(CANCEL_REQUEST_CODE.to_string(), "Cancel Request")
    }

    #[test]
    fn ssl_request() {
        assert_eq!(SSL_REQUEST_CODE.to_string(), "SSL Request")
    }

    #[test]
    fn gssenc_request() {
        assert_eq!(GSSENC_REQUEST_CODE.to_string(), "GSSENC Request")
    }

    #[test]
    fn other_request() {
        assert_eq!(Code(0x11_22_33_44).to_string(), "High bytes 0x1122 Low bytes: 0x3344")
    }
}
