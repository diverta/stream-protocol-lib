use serde_json::{Map, Value};

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusArray, StatusBool, StatusNull, StatusNumber, StatusObject, StatusString, StatusTrait};

#[derive(Debug, PartialEq)]
pub(crate) struct StatusNone {
    
}

impl StatusTrait for StatusNone {
    fn new() -> Self {
        Self {}
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match c {
            b' ' | b'\n' | b'\r' => {
                // Any whitespace or newline is absorbed without state change
                return Ok(None)
            },
            b'"' => {
                // Beginning of a new value as a String
                return Ok(Some((Some(Value::String(String::new())), Some(Status::String(StatusString::new())))))
            },
            b'n' => {
                // Beginning of a new value as null : output only at the last character, not now
                return Ok(Some((None, Some(Status::Null(StatusNull::new().with_char(c))))))
            },
            b't' | b'f' => {
                // Beginning of a new value as Boolean
                return Ok(Some((None, Some(Status::Bool(StatusBool::new().with_char(c))))))
            },
            b'-' | b'0'..=b'9' => {
                // Beginning of a new value as Number true
                return Ok(Some((None, Some(Status::Number(StatusNumber::new().with_char(c))))))
            },
            b'{' => {
                // Beginning of a new value as Object
                return Ok(Some((Some(Value::Object(Map::new())), Some(Status::Object(StatusObject::new())))))
            },
            b'[' => {
                // Beginning of a new value as Array
                return Ok(Some((Some(Value::Array(Vec::new())), Some(Status::Array(StatusArray::new())))))
            },
            _ => return Err(ParseError::new("Top level object is not a valid JSON"))
        }
    }
    
    fn flush(&mut self) -> Option<Value> {
        None
    }
    
    fn with_char(self, _c: &u8) -> Self {
        unreachable!("None must not be initialized with_char")
    }
}