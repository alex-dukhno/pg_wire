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

use crate::{cursor::Cursor, Oid, PgFormat};
use std::{
    convert::TryFrom,
    fmt::{self, Display, Formatter},
};

/// Represents PostgreSQL data type and methods to send over wire
#[derive(Debug, PartialEq, Clone)]
pub enum PgType {
    /// Represents PostgreSQL `smallint` data type
    SmallInt,
    /// Represents PostgreSQL `integer` (or `int`) data type
    Integer,
    /// Represents PostgreSQL `bigint` data type
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
            Self::Bool => 1,
            Self::Char => 1,
            Self::BigInt => 8,
            Self::SmallInt => 2,
            Self::Integer => 4,
            Self::VarChar => -1,
        }
    }

    /// Deserializes a value of this type from `raw` using the specified `format`.
    pub fn decode(&self, format: &PgFormat, raw: &[u8]) -> Result<Value, String> {
        match format {
            PgFormat::Binary => self.decode_binary(&mut Cursor::from(raw)),
            PgFormat::Text => self.decode_text(raw),
        }
    }

    fn decode_binary(&self, raw: &mut Cursor) -> Result<Value, String> {
        match self {
            Self::Bool => parse_bool_from_binary(raw),
            Self::Char => parse_char_from_binary(raw),
            Self::VarChar => parse_varchar_from_binary(raw),
            Self::SmallInt => parse_smallint_from_binary(raw),
            Self::Integer => parse_integer_from_binary(raw),
            Self::BigInt => parse_bigint_from_binary(raw),
        }
    }

    fn decode_text(&self, raw: &[u8]) -> Result<Value, String> {
        let s = match std::str::from_utf8(raw) {
            Ok(s) => s,
            Err(_) => return Err(format!("Failed to parse UTF8 from: {:?}", raw)),
        };

        match self {
            Self::Bool => parse_bool_from_text(s),
            Self::Char => parse_char_from_text(s),
            Self::VarChar => parse_varchar_from_text(s),
            Self::SmallInt => parse_smallint_from_text(s),
            Self::Integer => parse_integer_from_text(s),
            Self::BigInt => parse_bigint_from_text(s),
        }
    }
}

impl Display for PgType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool => write!(f, "boolean"),
            Self::Char => write!(f, "character"),
            Self::BigInt => write!(f, "bigint"),
            Self::SmallInt => write!(f, "smallint"),
            Self::Integer => write!(f, "integer"),
            Self::VarChar => write!(f, "variable character"),
        }
    }
}

/// Not supported OID
#[derive(Debug, PartialEq)]
pub struct NotSupportedOid(pub(crate) Oid);

impl TryFrom<Oid> for PgType {
    type Error = NotSupportedOid;

    /// Returns the type corresponding to the provided [Oid], if the it is known.
    fn try_from(oid: Oid) -> Result<Self, Self::Error> {
        match oid {
            16 => Ok(PgType::Bool),
            18 => Ok(PgType::Char),
            20 => Ok(PgType::BigInt),
            21 => Ok(PgType::SmallInt),
            23 => Ok(PgType::Integer),
            1043 => Ok(PgType::VarChar),
            _ => Err(NotSupportedOid(oid)),
        }
    }
}

fn parse_bigint_from_binary(buf: &mut Cursor) -> Result<Value, String> {
    let v = match buf.read_i64() {
        Ok(v) => v,
        Err(_) => return Err(format!("Failed to parse BigInt from: {:?}", buf)),
    };

    Ok(Value::Int64(v))
}

fn parse_bigint_from_text(s: &str) -> Result<Value, String> {
    let v: i64 = match s.trim().parse() {
        Ok(v) => v,
        Err(_) => return Err(format!("Failed to parse SmallInt from: {}", s)),
    };

    Ok(Value::Int64(v))
}

fn parse_bool_from_binary(buf: &mut Cursor) -> Result<Value, String> {
    let v = match buf.read_byte() {
        Ok(0) => Value::False,
        Ok(_) => Value::True,
        _ => return Err("invalid buffer size".into()),
    };

    Ok(v)
}

fn parse_bool_from_text(s: &str) -> Result<Value, String> {
    match s.trim().to_lowercase().as_str() {
        "t" | "tr" | "tru" | "true" | "y" | "ye" | "yes" | "on" | "1" => Ok(Value::True),
        "f" | "fa" | "fal" | "fals" | "false" | "n" | "no" | "of" | "off" | "0" => Ok(Value::False),
        _ => Err(format!("Failed to parse Bool from: {}", s)),
    }
}

fn parse_char_from_binary(buf: &mut Cursor) -> Result<Value, String> {
    let s = match buf.read_str() {
        Ok(s) => s,
        Err(_) => return Err(format!("Failed to parse UTF8 from: {:?}", buf)),
    };

    Ok(Value::String(s.into()))
}

fn parse_char_from_text(s: &str) -> Result<Value, String> {
    Ok(Value::String(s.into()))
}

fn parse_integer_from_binary(buf: &mut Cursor) -> Result<Value, String> {
    let v = match buf.read_i32() {
        Ok(v) => v,
        Err(_) => return Err(format!("Failed to parse Integer from: {:?}", buf)),
    };

    Ok(Value::Int32(v))
}

fn parse_integer_from_text(s: &str) -> Result<Value, String> {
    let v: i32 = match s.trim().parse() {
        Ok(v) => v,
        Err(_) => return Err(format!("Failed to parse SmallInt from: {}", s)),
    };

    Ok(Value::Int32(v))
}

fn parse_smallint_from_binary(buf: &mut Cursor) -> Result<Value, String> {
    let v = match buf.read_i32() {
        Ok(v) => v as i16,
        Err(_) => return Err(format!("Failed to parse SmallInt from: {:?}", buf)),
    };

    Ok(Value::Int16(v))
}

fn parse_smallint_from_text(s: &str) -> Result<Value, String> {
    let v: i16 = match s.trim().parse() {
        Ok(v) => v,
        Err(_) => return Err(format!("Failed to parse SmallInt from: {}", s)),
    };

    Ok(Value::Int16(v))
}

fn parse_varchar_from_binary(buf: &mut Cursor) -> Result<Value, String> {
    let s = match buf.read_str() {
        Ok(s) => s,
        Err(_) => return Err(format!("Failed to parse UTF8 from: {:?}", buf)),
    };

    Ok(Value::String(s.into()))
}

fn parse_varchar_from_text(s: &str) -> Result<Value, String> {
    Ok(Value::String(s.into()))
}

/// Represents PostgreSQL data values sent and received over wire
#[allow(missing_docs)]
#[derive(Debug, PartialEq)]
pub enum Value {
    Null,
    True,
    False,
    Int16(i16),
    Int32(i32),
    Int64(i64),
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
            assert_eq!(PgType::try_from(1_000_000), Err(NotSupportedOid(1_000_000)));
        }

        #[test]
        fn boolean() {
            assert_eq!(PgType::Bool.type_oid(), 16);
            assert_eq!(PgType::try_from(PgType::Bool.type_oid()), Ok(PgType::Bool));
        }

        #[test]
        fn character() {
            assert_eq!(PgType::Char.type_oid(), 18);
            assert_eq!(PgType::try_from(PgType::Char.type_oid()), Ok(PgType::Char));
        }

        #[test]
        fn big_int() {
            assert_eq!(PgType::BigInt.type_oid(), 20);
            assert_eq!(PgType::try_from(PgType::BigInt.type_oid()), Ok(PgType::BigInt));
        }

        #[test]
        fn small_int() {
            assert_eq!(PgType::SmallInt.type_oid(), 21);
            assert_eq!(PgType::try_from(PgType::SmallInt.type_oid()), Ok(PgType::SmallInt));
        }

        #[test]
        fn integer() {
            assert_eq!(PgType::Integer.type_oid(), 23);
            assert_eq!(PgType::try_from(PgType::Integer.type_oid()), Ok(PgType::Integer));
        }

        #[test]
        fn variable_characters() {
            assert_eq!(PgType::VarChar.type_oid(), 1043);
            assert_eq!(PgType::try_from(PgType::VarChar.type_oid()), Ok(PgType::VarChar));
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
            assert_eq!(PgType::Bool.decode(&PgFormat::Binary, &[1]), Ok(Value::True));
        }

        #[test]
        fn decode_false() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Binary, &[0]), Ok(Value::False));
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
        fn decode_smallint() {
            assert_eq!(
                PgType::SmallInt.decode(&PgFormat::Binary, &[0, 0, 0, 1]),
                Ok(Value::Int16(1))
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
        fn decode_bigint() {
            assert_eq!(
                PgType::BigInt.decode(&PgFormat::Binary, &[0, 0, 0, 0, 0, 0, 0, 1]),
                Ok(Value::Int64(1))
            );
        }
    }

    #[cfg(test)]
    mod text_decoding {
        use super::*;

        #[test]
        fn error_decode_text() {
            assert_eq!(
                PgType::Bool.decode(&PgFormat::Text, &[0x96]),
                Err("Failed to parse UTF8 from: [150]".into())
            );
        }

        #[test]
        fn decode_true() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Text, b"true"), Ok(Value::True));
        }

        #[test]
        fn decode_false() {
            assert_eq!(PgType::Bool.decode(&PgFormat::Text, b"0"), Ok(Value::False));
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
        fn decode_smallint() {
            assert_eq!(PgType::SmallInt.decode(&PgFormat::Text, b"1"), Ok(Value::Int16(1)));
        }

        #[test]
        fn decode_integer() {
            assert_eq!(PgType::Integer.decode(&PgFormat::Text, b"123"), Ok(Value::Int32(123)));
        }

        #[test]
        fn decode_bigint() {
            assert_eq!(
                PgType::BigInt.decode(&PgFormat::Text, b"123456"),
                Ok(Value::Int64(123456))
            );
        }
    }
}
