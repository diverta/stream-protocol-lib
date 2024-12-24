use serde_json::{Map, Value};

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusBool, StatusDone, StatusNull, StatusNumber, StatusObject, StatusString, StatusTrait};

#[derive(Debug)]
pub(crate) struct StatusArray {
    pub(crate) comma_matched: bool,
}

impl StatusTrait for StatusArray {
    fn new() -> Self {
        Self {
            comma_matched: true // Start as if after comma
        }
    }
    
    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (self.comma_matched, c) {
            (_, b' ' | b'\n' | b'\r') => {
                // Any whitespace or newline is absorbed without state change
                return Ok(None)
            },
            (false, b',') => {
                self.comma_matched = true;
                return Ok(None)
            },
            (true, b',') => {
                return Err(ParseError::new("Double comma inside array"))
            },
            (true, b'"') => {
                // Beginning of a new value as a String
                return Ok(Some((Some(Value::String(String::new())), Some(Status::String(StatusString::new())))))
            },
            (true, b'n') => {
                // Beginning of a new value as null
                return Ok(Some((Some(Value::Null), Some(Status::Null(StatusNull::new().with_char(c))))))
            },
            (true, b't' | b'f') => {
                // Beginning of a new value as Boolean
                return Ok(Some((None, Some(Status::Bool(StatusBool::new().with_char(c))))))
            },
            (true, b'-' | b'0'..=b'9') => {
                // Beginning of a new value as Number true
                return Ok(Some((None, Some(Status::Number(StatusNumber::new().with_char(c))))))
            },
            (true, b'{') => {
                // Beginning of a new value as Object
                return Ok(Some((Some(Value::Object(Map::new())), Some(Status::Object(StatusObject::new())))))
            },
            (true, b'[') => {
                // Beginning of a new value as Array
                return Ok(Some((Some(Value::Array(Vec::new())), Some(Status::Array(StatusArray::new())))))
            },
            (_, b']') => {
                // Closing the array
                return Ok(Some((None, Some(Status::Done(StatusDone::default())))));
            },
            _ => return Err(ParseError::new("Invalid value in array"))
        }
    }
    
    fn flush(&mut self) -> Option<Vec<u8>> {
        None
    }
    
    fn with_char(self, _c: &u8) -> Self {
        unreachable!("Array must not be initialized with_char")
    }
}