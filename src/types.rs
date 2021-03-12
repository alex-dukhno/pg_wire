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

use crate::{
    errors::{TypeValueDecodeErrorKind},
    Oid, PgFormat, TypeValueDecodeError,
};
use std::{
    fmt::{self, Display, Formatter},
    str,
};
use crate::errors::NotSupportedOid;

const BOOL_TRUE: &[&str] = &["t", "tr", "tru", "true", "y", "ye", "yes", "on", "1"];
const BOOL_FALSE: &[&str] = &["f", "fa", "fal", "fals", "false", "n", "no", "of", "off", "0"];

/// Represents PostgreSQL data type and methods to send over wire
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PgType {
    /// Represents PostgreSQL `smallint` (or `int2`) data type
    SmallInt,
    /// Represents PostgreSQL `integer` (or `int` or `int4`) data type
    Integer,
    /// Represents PostgreSQL `bigint` (or `int8`) data type
    BigInt,

    /// Represents PostgreSQL `character(n)` (or `char(n)`) data type
    Char,
    /// Represents PostgreSQL `character varying(n)` (or `varchar(n)`) data type
    VarChar,

    /// Represents PostgreSQL `boolean` data type
    Bool,
}

impl PgType {
    /// Returns PostgreSQL type [Oid](Oid)
    pub fn type_oid(&self) -> Oid {
        match self {
            PgType::Bool => 16,
            PgType::Char => 18,
            PgType::BigInt => 20,
            PgType::SmallInt => 21,
            PgType::Integer => 23,
            PgType::VarChar => 1043,
        }
    }

    /// Returns PostgreSQL type length
    pub fn type_len(&self) -> i16 {
        match self {
            PgType::Bool => 1,
            PgType::Char => 1,
            PgType::BigInt => 8,
            PgType::SmallInt => 2,
            PgType::Integer => 4,
            PgType::VarChar => -1,
        }
    }

    /// Deserializes a value of this type from `raw` using the specified `format`.
    pub fn decode<'d>(&'d self, format: &'d PgFormat, raw: &'d [u8]) -> Result<Value, TypeValueDecodeError<'d>> {
        match format {
            PgFormat::Binary => self.decode_binary(raw).map_err(Into::into),
            PgFormat::Text => self.decode_text(raw).map_err(Into::into),
        }
    }

    /// Returns the type corresponding to the provided [Oid], if the it is known.
    pub fn from_oid(oid: Oid) -> Result<Option<PgType>, NotSupportedOid> {
        match oid {
            0 => Ok(None),
            16 => Ok(Some(PgType::Bool)),
            18 => Ok(Some(PgType::Char)),
            20 => Ok(Some(PgType::BigInt)),
            21 => Ok(Some(PgType::SmallInt)),
            23 => Ok(Some(PgType::Integer)),
            1043 => Ok(Some(PgType::VarChar)),
            _ => Err(NotSupportedOid(oid)),
        }
    }

    fn decode_binary<'d>(&'d self, raw: &'d [u8]) -> Result<Value, TypeValueDecodeErrorKind<'d>> {
        match self {
            PgType::Bool => {
                if raw.is_empty() {
                    Err(TypeValueDecodeErrorKind::NotEnoughBytes {
                        required_bytes: 1,
                        source: raw,
                        pg_type: *self,
                    })
                } else {
                    Ok(Value::Bool(raw[0] != 0))
                }
            }
            PgType::Char | PgType::VarChar => str::from_utf8(raw)
                .map(|s| Value::String(s.into()))
                .map_err(|cause| TypeValueDecodeErrorKind::CannotDecodeString { cause, source: raw }),
            PgType::SmallInt => {
                if raw.len() < 4 {
                    Err(TypeValueDecodeErrorKind::NotEnoughBytes {
                        required_bytes: 4,
                        source: raw,
                        pg_type: *self,
                    })
                } else {
                    Ok(Value::Int16(i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as i16))
                }
            }
            PgType::Integer => {
                if raw.len() < 4 {
                    Err(TypeValueDecodeErrorKind::NotEnoughBytes {
                        required_bytes: 4,
                        source: raw,
                        pg_type: *self,
                    })
                } else {
                    Ok(Value::Int32(i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]])))
                }
            }
            PgType::BigInt => {
                if raw.len() < 8 {
                    Err(TypeValueDecodeErrorKind::NotEnoughBytes {
                        required_bytes: 8,
                        source: raw,
                        pg_type: *self,
                    })
                } else {
                    Ok(Value::Int64(i64::from_be_bytes([
                        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
                    ])))
                }
            }
        }
    }

    fn decode_text<'d>(&'d self, raw: &'d [u8]) -> Result<Value, TypeValueDecodeErrorKind<'d>> {
        let s = match str::from_utf8(raw) {
            Ok(s) => s,
            Err(cause) => return Err(TypeValueDecodeErrorKind::CannotDecodeString { cause, source: raw }),
        };

        match self {
            PgType::Bool => {
                let v = s.trim().to_lowercase();
                if BOOL_TRUE.contains(&v.as_str()) {
                    Ok(Value::Bool(true))
                } else if BOOL_FALSE.contains(&v.as_str()) {
                    Ok(Value::Bool(false))
                } else {
                    Err(TypeValueDecodeErrorKind::CannotParseBool { source: s })
                }
            }
            PgType::Char => Ok(Value::String(s.into())),
            PgType::VarChar => Ok(Value::String(s.into())),
            PgType::SmallInt => {
                s.trim()
                    .parse()
                    .map(Value::Int16)
                    .map_err(|cause| TypeValueDecodeErrorKind::CannotParseInt {
                        cause,
                        source: s,
                        pg_type: *self,
                    })
            }
            PgType::Integer => {
                s.trim()
                    .parse()
                    .map(Value::Int32)
                    .map_err(|cause| TypeValueDecodeErrorKind::CannotParseInt {
                        cause,
                        source: s,
                        pg_type: *self,
                    })
            }
            PgType::BigInt => {
                s.trim()
                    .parse()
                    .map(Value::Int64)
                    .map_err(|cause| TypeValueDecodeErrorKind::CannotParseInt {
                        cause,
                        source: s,
                        pg_type: *self,
                    })
            }
        }
    }
}

impl Display for PgType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PgType::Bool => write!(f, "boolean"),
            PgType::Char => write!(f, "character"),
            PgType::BigInt => write!(f, "bigint"),
            PgType::SmallInt => write!(f, "smallint"),
            PgType::Integer => write!(f, "integer"),
            PgType::VarChar => write!(f, "variable character"),
        }
    }
}

/// Represents PostgreSQL data values sent and received over wire
#[allow(missing_docs)]
#[derive(Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    /// Supports only UTF-8 encoding
    String(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod types_oid {
        use super::*;

        #[test]
        fn not_supported_oid() {
            assert_eq!(PgType::from_oid(1_000_000), Err(NotSupportedOid(1_000_000)));
        }

        #[test]
        fn undefined() {
            assert_eq!(PgType::from_oid(0), Ok(None));
        }

        #[test]
        fn boolean() {
            assert_eq!(PgType::Bool.type_oid(), 16);
            assert_eq!(PgType::from_oid(PgType::Bool.type_oid()), Ok(Some(PgType::Bool)));
        }

        #[test]
        fn character() {
            assert_eq!(PgType::Char.type_oid(), 18);
            assert_eq!(PgType::from_oid(PgType::Char.type_oid()), Ok(Some(PgType::Char)));
        }

        #[test]
        fn big_int() {
            assert_eq!(PgType::BigInt.type_oid(), 20);
            assert_eq!(PgType::from_oid(PgType::BigInt.type_oid()), Ok(Some(PgType::BigInt)));
        }

        #[test]
        fn small_int() {
            assert_eq!(PgType::SmallInt.type_oid(), 21);
            assert_eq!(
                PgType::from_oid(PgType::SmallInt.type_oid()),
                Ok(Some(PgType::SmallInt))
            );
        }

        #[test]
        fn integer() {
            assert_eq!(PgType::Integer.type_oid(), 23);
            assert_eq!(PgType::from_oid(PgType::Integer.type_oid()), Ok(Some(PgType::Integer)));
        }

        #[test]
        fn variable_characters() {
            assert_eq!(PgType::VarChar.type_oid(), 1043);
            assert_eq!(PgType::from_oid(PgType::VarChar.type_oid()), Ok(Some(PgType::VarChar)));
        }
    }

    #[cfg(test)]
    mod types_length {
        use super::*;

        #[test]
        fn boolean() {
            assert_eq!(PgType::Bool.type_len(), 1);
        }

        #[test]
        fn character() {
            assert_eq!(PgType::Char.type_len(), 1);
        }

        #[test]
        fn big_int() {
            assert_eq!(PgType::BigInt.type_len(), 8);
        }

        #[test]
        fn small_int() {
            assert_eq!(PgType::SmallInt.type_len(), 2);
        }

        #[test]
        fn integer() {
            assert_eq!(PgType::Integer.type_len(), 4);
        }

        #[test]
        fn variable_characters() {
            assert_eq!(PgType::VarChar.type_len(), -1);
        }
    }

    #[cfg(test)]
    mod type_string_representation {
        use super::*;

        #[test]
        fn boolean() {
            assert_eq!(PgType::Bool.to_string(), "boolean".to_string());
        }

        #[test]
        fn character() {
            assert_eq!(PgType::Char.to_string(), "character".to_string());
        }

        #[test]
        fn big_int() {
            assert_eq!(PgType::BigInt.to_string(), "bigint".to_string());
        }

        #[test]
        fn small_int() {
            assert_eq!(PgType::SmallInt.to_string(), "smallint".to_string());
        }

        #[test]
        fn integer() {
            assert_eq!(PgType::Integer.to_string(), "integer".to_string());
        }

        #[test]
        fn variable_characters() {
            assert_eq!(PgType::VarChar.to_string(), "variable character".to_string());
        }
    }

    #[cfg(test)]
    mod binary_decoding {
        use super::*;

        #[test]
        fn decode_true() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Binary, &[1]), Ok(Value::Bool(true)));
        }

        #[test]
        fn decode_false() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Binary, &[0]), Ok(Value::Bool(false)));
        }

        #[test]
        fn error_decode_bool() {
            assert_eq!(
                PgType::Bool.decode(&PgFormat::Binary, &[]),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::NotEnoughBytes {
                    required_bytes: 1,
                    source: &[],
                    pg_type: PgType::Bool
                }))
            );
        }

        #[test]
        fn decode_char() {
            assert_eq!(
                PgType::Char.decode(&PgFormat::Binary, &[97, 98, 99]),
                Ok(Value::String("abc".into()))
            );
        }

        #[test]
        fn decode_varchar() {
            assert_eq!(
                PgType::VarChar.decode(&PgFormat::Binary, &[97, 98, 99]),
                Ok(Value::String("abc".into()))
            );
        }

        #[test]
        fn error_decode_string() {
            let non_utf_code = 0x96;
            assert_eq!(
                PgType::Char.decode(&PgFormat::Binary, &[non_utf_code]),
                Err(TypeValueDecodeError::from(
                    TypeValueDecodeErrorKind::CannotDecodeString {
                        cause: str::from_utf8(&[non_utf_code]).unwrap_err(),
                        source: &[non_utf_code]
                    }
                ))
            );
        }

        #[test]
        fn decode_smallint() {
            assert_eq!(
                PgType::SmallInt.decode(&PgFormat::Binary, &[0, 0, 0, 1]),
                Ok(Value::Int16(1))
            );
        }

        #[test]
        fn error_decode_smallint() {
            assert_eq!(
                PgType::SmallInt.decode(&PgFormat::Binary, &[0, 0, 1]),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::NotEnoughBytes {
                    required_bytes: 4,
                    source: &[0, 0, 1],
                    pg_type: PgType::SmallInt
                }))
            );
        }

        #[test]
        fn decode_integer() {
            assert_eq!(
                PgType::Integer.decode(&PgFormat::Binary, &[0, 0, 0, 1]),
                Ok(Value::Int32(1))
            );
        }

        #[test]
        fn error_decode_integer() {
            assert_eq!(
                PgType::Integer.decode(&PgFormat::Binary, &[0, 0, 1]),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::NotEnoughBytes {
                    required_bytes: 4,
                    source: &[0, 0, 1],
                    pg_type: PgType::Integer
                }))
            );
        }

        #[test]
        fn decode_bigint() {
            assert_eq!(
                PgType::BigInt.decode(&PgFormat::Binary, &[0, 0, 0, 0, 0, 0, 0, 1]),
                Ok(Value::Int64(1))
            );
        }

        #[test]
        fn error_decode_bigint() {
            assert_eq!(
                PgType::BigInt.decode(&PgFormat::Binary, &[0, 0, 1]),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::NotEnoughBytes {
                    required_bytes: 8,
                    source: &[0, 0, 1],
                    pg_type: PgType::BigInt
                }))
            );
        }
    }

    #[cfg(test)]
    mod text_decoding {
        use std::str::FromStr;

        use super::*;

        #[test]
        fn decode_true() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Text, b"true"), Ok(Value::Bool(true)));
        }

        #[test]
        fn decode_false() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Text, b"0"), Ok(Value::Bool(false)));
        }

        #[test]
        fn error_decode_bool() {
            assert_eq!(
                PgType::Bool.decode(&PgFormat::Text, b"abc"),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::CannotParseBool {
                    source: "abc"
                }))
            );
        }

        #[test]
        fn decode_char() {
            assert_eq!(
                PgType::Char.decode(&PgFormat::Text, b"abc"),
                Ok(Value::String("abc".into()))
            );
        }

        #[test]
        fn decode_varchar() {
            assert_eq!(
                PgType::VarChar.decode(&PgFormat::Text, b"abc"),
                Ok(Value::String("abc".into()))
            );
        }

        #[test]
        fn error_decode_string() {
            let non_utf_code = 0x96;
            assert_eq!(
                PgType::Char.decode(&PgFormat::Text, &[non_utf_code]),
                Err(TypeValueDecodeError::from(
                    TypeValueDecodeErrorKind::CannotDecodeString {
                        cause: str::from_utf8(&[non_utf_code]).unwrap_err(),
                        source: &[non_utf_code]
                    }
                ))
            );
        }

        #[test]
        fn decode_smallint() {
            assert_eq!(PgType::SmallInt.decode(&PgFormat::Text, b"1"), Ok(Value::Int16(1)));
        }

        #[test]
        fn error_decode_smallint() {
            assert_eq!(
                PgType::SmallInt.decode(&PgFormat::Text, b"1.0"),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::CannotParseInt {
                    cause: i16::from_str("1.0").unwrap_err(),
                    source: &"1.0",
                    pg_type: PgType::SmallInt
                }))
            );
        }

        #[test]
        fn decode_integer() {
            assert_eq!(PgType::Integer.decode(&PgFormat::Text, b"123"), Ok(Value::Int32(123)));
        }

        #[test]
        fn error_decode_integer() {
            assert_eq!(
                PgType::Integer.decode(&PgFormat::Text, b"1.0"),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::CannotParseInt {
                    cause: i32::from_str("1.0").unwrap_err(),
                    source: &"1.0",
                    pg_type: PgType::Integer
                }))
            );
        }

        #[test]
        fn decode_bigint() {
            assert_eq!(
                PgType::BigInt.decode(&PgFormat::Text, b"123456"),
                Ok(Value::Int64(123456))
            );
        }

        #[test]
        fn error_decode_bigint() {
            assert_eq!(
                PgType::BigInt.decode(&PgFormat::Text, b"1.0"),
                Err(TypeValueDecodeError::from(TypeValueDecodeErrorKind::CannotParseInt {
                    cause: i64::from_str("1.0").unwrap_err(),
                    source: &"1.0",
                    pg_type: PgType::BigInt
                }))
            );
        }
    }
}
