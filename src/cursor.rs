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

use crate::{Error, Result};

/// Cursor over `u8` slice to decode it into primitive types from big endian format
#[derive(Debug)]
pub(crate) struct Cursor<'a> {
    buf: &'a [u8],
}

impl<'c> From<&'c [u8]> for Cursor<'c> {
    fn from(buf: &'c [u8]) -> Cursor {
        Cursor { buf }
    }
}

impl<'c> From<&'c Cursor<'c>> for Vec<u8> {
    fn from(cur: &'c Cursor<'c>) -> Vec<u8> {
        cur.buf.to_vec()
    }
}

impl<'a> Cursor<'a> {
    fn advance(&mut self, n: usize) {
        self.buf = &self.buf[n..]
    }

    fn peek_byte(&self) -> Result<u8> {
        self.buf
            .get(0)
            .copied()
            .ok_or_else(|| Error::InvalidInput("No byte to read".to_owned()))
    }

    pub(crate) fn read_byte(&mut self) -> Result<u8> {
        let byte = self.peek_byte()?;
        self.advance(1);
        Ok(byte)
    }

    /// Returns the next null-terminated string. The null character is not
    /// included the returned string. The cursor is advanced past the null-
    /// terminated string.
    pub(crate) fn read_cstr(&mut self) -> Result<&'a str> {
        if let Some(pos) = self.buf.iter().position(|b| *b == 0) {
            let val = std::str::from_utf8(&self.buf[..pos]).map_err(|_e| Error::InvalidUtfString)?;
            self.advance(pos + 1);
            Ok(val)
        } else {
            Err(Error::ZeroByteNotFound)
        }
    }

    pub(crate) fn read_str(&mut self) -> Result<&'a str> {
        let val = std::str::from_utf8(&self.buf).map_err(|_e| Error::InvalidUtfString)?;
        self.advance(self.buf.len());
        Ok(val)
    }

    /// Reads the next 16-bit signed integer, advancing the cursor by two
    /// bytes.
    pub(crate) fn read_i16(&mut self) -> Result<i16> {
        if self.buf.len() < 2 {
            return Err(Error::InvalidInput("not enough buffer to read 16bit Int".to_owned()));
        }
        let val = i16::from_be_bytes([self.buf[0], self.buf[1]]);
        self.advance(2);
        Ok(val)
    }

    /// Reads the next 32-bit signed integer, advancing the cursor by four
    /// bytes.
    pub(crate) fn read_i32(&mut self) -> Result<i32> {
        if self.buf.len() < 4 {
            return Err(Error::InvalidInput("not enough buffer to read 32bit Int".to_owned()));
        }
        let val = i32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
        self.advance(4);
        Ok(val)
    }

    /// Reads the next 32-bit unsigned integer, advancing the cursor by four
    /// bytes.
    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        self.read_i32().map(|val| val as u32)
    }

    /// Reads the next 64-bit signed integer, advancing the cursor by eight
    /// bytes.
    pub(crate) fn read_i64(&mut self) -> Result<i64> {
        if self.buf.len() < 8 {
            return Err(Error::InvalidInput("not enough buffer to read 64bit Int".to_owned()));
        }
        let val = i64::from_be_bytes([
            self.buf[0],
            self.buf[1],
            self.buf[2],
            self.buf[3],
            self.buf[4],
            self.buf[5],
            self.buf[6],
            self.buf[7],
        ]);
        self.advance(8);
        Ok(val)
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
            Err(Error::InvalidInput("No byte to read".to_owned()))
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
        assert_eq!(cursor.read_cstr(), Err(Error::ZeroByteNotFound));
    }

    #[test]
    fn invalid_utf_read_cstr() {
        let invalid_utf_byte = 0x96;
        let mut buffer = b"some string".to_vec();
        buffer.push(invalid_utf_byte); // invalid utf byte
        buffer.push(0);
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_cstr(), Err(Error::InvalidUtfString));
    }

    #[test]
    fn ok_read_str() {
        let buffer = b"some string".to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_str(), Ok("some string"));
    }

    #[test]
    fn invalid_utf_read_str() {
        let invalid_utf_byte = 0x96;
        let mut buffer = b"some string".to_vec();
        buffer.push(invalid_utf_byte); // invalid utf byte
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_str(), Err(Error::InvalidUtfString));
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
            Err(Error::InvalidInput("not enough buffer to read 16bit Int".into()))
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
            Err(Error::InvalidInput("not enough buffer to read 32bit Int".into()))
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
            Err(Error::InvalidInput("not enough buffer to read 32bit Int".into()))
        );
    }

    #[test]
    fn ok_read_i64() {
        let buffer = 123i64.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(cursor.read_i64(), Ok(123));
    }

    #[test]
    fn error_read_i64() {
        let buffer = 123i16.to_be_bytes().to_vec();
        let mut cursor = Cursor::from(buffer.as_slice());
        assert_eq!(
            cursor.read_i64(),
            Err(Error::InvalidInput("not enough buffer to read 64bit Int".into()))
        );
    }
}
