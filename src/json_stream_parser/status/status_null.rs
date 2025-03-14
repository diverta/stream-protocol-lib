use serde_json::Value;

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusDone, StatusTrait};

#[derive(Debug, PartialEq)]
pub struct StatusNull {
    match_so_far: Vec<u8> // Contains incomplete sequence
}

impl StatusTrait for StatusNull {
    fn new() -> Self {
        Self {
            match_so_far: Vec::new()
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (&self.match_so_far[..], c) {
            ([], b'n') |
            ([b'n'], b'u') |
            ([b'n', b'u'], b'l')
            => {
                self.match_so_far.push(*c);
                return Ok(None);
            },
            ([b'n',b'u',b'l'], b'l')
            => {
                self.match_so_far.clear();
                return Ok(Some((Some(Value::Null), Some(Status::Done(StatusDone::default())))))
            }
            _ => {
                return Err(ParseError::new("Invalid null"))
            }
        }
    }

    fn flush(&mut self) -> Option<Value> {
        None
    }
    
    fn with_char(mut self, c: &u8) -> Self {
        let _ = self.add_char(c); // For initialization
        self
    }
}