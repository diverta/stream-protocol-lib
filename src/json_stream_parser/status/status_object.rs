use serde_json::{Map, Value};

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusArray, StatusBool, StatusNull, StatusNumber, StatusString, StatusTrait};

#[derive(Debug)]
pub(crate) enum SubStatusObject {
    BeforeKV(bool), // true if ',' is matched
    BetweenKV(bool), // true if ':' is matched
}

#[derive(Debug)]
pub(crate) struct StatusObject {
    pub(crate) substatus: SubStatusObject,
}

impl StatusTrait for StatusObject {
    fn new() -> Self {
        Self {
            substatus: SubStatusObject::BeforeKV(true) // At start, comma is skipped (considered matched)
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match self.substatus {
            SubStatusObject::BeforeKV(comma_matched) => {
                match c {
                    b' ' | b'\n' | b'\r' => {
                        // Any whitespace or newline is absorbed without state change
                        return Ok(None)
                    },
                    b'"' => {
                        // Beginning of a new value as a String
                        return Ok(Some((Some(Value::String(String::new())), Some(Status::String(StatusString::new())))))
                    },
                    b',' => {
                        if !comma_matched {
                            self.substatus = SubStatusObject::BeforeKV(true);
                            return Ok(None)
                        } else {
                            return Err(ParseError::new("Double comma inside object"))
                        }
                    },
                    b'}' => {
                        if !comma_matched {
                            // Closing the object
                            self.substatus = SubStatusObject::BeforeKV(true);
                            return Ok(Some((None, Some(Status::Done(super::StatusDone::default())))));
                        } else {
                            return Err(ParseError::new("Closing bracket after a comma in an object"))
                        }
                    },
                    _ => return Err(ParseError::new("Key double quote expected in object"))
                }
            },
            SubStatusObject::BetweenKV(colon_matched) => {
                match c {
                    b' ' | b'\n' | b'\r' => {
                        // Any whitespace or newline is absorbed without state change
                        return Ok(None)
                    },
                    b':' => {
                        if !colon_matched {
                            self.substatus = SubStatusObject::BetweenKV(true)
                        } else {
                            return Err(ParseError::new(""))
                        }
                        return Ok(None)
                    },
                    b'"' => {
                        // Beginning of a new value as a String
                        return Ok(Some((Some(Value::String(String::new())), Some(Status::String(StatusString::new())))))
                    },
                    b'n' => {
                        // Beginning of a new value as null
                        return Ok(Some((Some(Value::Null), Some(Status::Null(StatusNull::new().with_char(c))))))
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
                    _ => return Err(ParseError::new("Object does not have a valid value"))
                }
            }
        }
    }

    fn flush(&mut self) -> Option<Vec<u8>> {
        None
    }
    
    fn with_char(self, _c: &u8) -> Self {
        unreachable!("Object must not be initialized with_char")
    }
}