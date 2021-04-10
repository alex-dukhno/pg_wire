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

use crate::errors::{PayloadError, PayloadErrorKind};
use std::str;

/// Cursor over `u8` slice to decode it into primitive types from big endian format
#[derive(Debug)]
pub(crate) struct Cursor<'c> {
    buf: &'c [u8],
}

impl<'c> From<&'c [u8]> for Cursor<'c> {
    fn from(buf: &'c [u8]) -> Cursor {
        Cursor { buf }
    }
}

impl<'c> Cursor<'c> {
    fn advance(&mut self, n: usize) {
        self.buf = &self.buf[n..]
    }

    fn peek_byte(&self) -> Result<u8, PayloadError> {
        self.buf
            .get(0)
            .copied()
            .ok_or_else(|| PayloadError::from(PayloadErrorKind::EndOfBuffer))
    }

    pub(crate) fn read_byte(&mut self) -> Result<u8, PayloadError> {
        let byte = self.peek_byte()?;
        self.advance(1);
        Ok(byte)
    }

    fn consume_next(&mut self, size: usize) -> Result<&'c [u8], PayloadError> {
        if self.buf.len() < size {
            Err(PayloadError::from(PayloadErrorKind::NotEnoughBytes {
                required: size as u8,
                source: self.buf.to_vec(),
            }))
        } else {
            let buf = &self.buf[0..size];
            self.advance(size);
            Ok(buf)
        }
    }

    /// Returns the next null-terminated string. The null character is not
    /// included the returned string. The cursor is advanced past the null-
    /// terminated string.
    pub(crate) fn read_cstr(&mut self) -> Result<&'c str, PayloadError> {
        if let Some(pos) = self.buf.iter().position(|b| *b == 0) {
            let val = str::from_utf8(&self.buf[..pos]).map_err(|cause| {
                PayloadError::from(PayloadErrorKind::InvalidUtfString {
                    cause,
                    source: self.buf[..pos].to_vec(),
                })
            })?;
            self.advance(pos + 1);
            Ok(val)
        } else {
            Err(PayloadError::from(PayloadErrorKind::CStringNotTerminated {
                source: self.buf.to_vec(),
            }))
        }
    }

    /// Reads the next 16-bit signed integer, advancing the cursor by two
    /// bytes.
    pub(crate) fn read_i16(&mut self) -> Result<i16, PayloadError> {
        self.consume_next(2).map(|buf| i16::from_be_bytes([buf[0], buf[1]]))
    }

    /// Reads the next 32-bit signed integer, advancing the cursor by four
    /// bytes.
    pub(crate) fn read_i32(&mut self) -> Result<i32, PayloadError> {
        self.consume_next(4)
            .map(|buf| i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]))
    }

    /// Reads the next 32-bit unsigned integer, advancing the cursor by four
    /// bytes.
    pub(crate) fn read_u32(&mut self) -> Result<u32, PayloadError> {
        self.read_i32().map(|val| val as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_read_byte() {
        let buffer = vec![1];
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_byte(), Ok(1));
    }

    #[test]
    fn error_read_byte() {
        let buffer = vec![];
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_byte(),
            Err(PayloadError::from(PayloadErrorKind::EndOfBuffer))
        );
    }

    #[test]
    fn ok_read_cstr() {
        let buffer = b"some string\0".to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_cstr(), Ok("some string"));
    }

    #[test]
    fn error_read_cstr() {
        let buffer = b"some string".to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_cstr(),
            Err(PayloadError::from(PayloadErrorKind::CStringNotTerminated {
                source: buffer.to_vec()
            }))
        );
    }

    #[test]
    fn invalid_utf_read_cstr() {
        let invalid_utf_byte = 0x96;
        let mut buffer = b"some string".to_vec();
        buffer.push(invalid_utf_byte);
        buffer.push(0);
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_cstr(),
            Err(PayloadError::from(PayloadErrorKind::InvalidUtfString {
                cause: str::from_utf8(&buffer).unwrap_err(),
                source: buffer[..buffer.len() - 1].to_vec()
            }))
        );
    }

    #[test]
    fn ok_read_i16() {
        let buffer = 123i16.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_i16(), Ok(123));
    }

    #[test]
    fn error_read_i16() {
        let buffer = 123i8.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_i16(),
            Err(PayloadError::from(PayloadErrorKind::NotEnoughBytes {
                required: 2,
                source: buffer.to_vec()
            }))
        );
    }

    #[test]
    fn ok_read_i32() {
        let buffer = 123i32.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_i32(), Ok(123));
    }

    #[test]
    fn error_read_i32() {
        let buffer = 123i16.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_i32(),
            Err(PayloadError::from(PayloadErrorKind::NotEnoughBytes {
                required: 4,
                source: buffer.to_vec()
            }))
        );
    }

    #[test]
    fn ok_read_u32() {
        let buffer = 123u32.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_u32(), Ok(123));
    }

    #[test]
    fn error_read_u32() {
        let buffer = 123i16.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_u32(),
            Err(PayloadError::from(PayloadErrorKind::NotEnoughBytes {
                required: 4,
                source: buffer.to_vec()
            }))
        );
    }
}
