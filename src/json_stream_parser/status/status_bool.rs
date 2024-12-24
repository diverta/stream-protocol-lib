use serde_json::Value;

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusDone, StatusTrait};


#[derive(Debug)]
pub(crate) struct StatusBool {
    match_so_far: Vec<u8> // Contains incomplete sequence
}

impl StatusTrait for StatusBool {
    fn new() -> Self {
        Self {
            match_so_far: Vec::new()
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (&self.match_so_far[..], c) {
            ([], b't') |
            ([b't'], b'r') |
            ([b't', b'r'], b'u') |
            ([], b'f') |
            ([b'f'], b'a') |
            ([b'f',b'a'], b'l') |
            ([b'f',b'a',b'l'], b's')
            => {
                self.match_so_far.push(*c);
                return Ok(None);
            },
            ([b't',b'r',b'u'], b'e')
            => {
                self.match_so_far.clear();
                return Ok(Some((Some(Value::Bool(true)), Some(Status::Done(StatusDone::default())))))
            }
            ([b'f',b'a',b'l',b's'], b'e')
            => {
                self.match_so_far.clear();
                return Ok(Some((Some(Value::Bool(false)), Some(Status::Done(StatusDone::default())))))
            },
            _ => {
                return Err(ParseError::new("Invalid boolean"))
            }
        }
    }

    fn flush(&mut self) -> Option<Vec<u8>> {
        None
    }
    
    fn with_char(mut self, c: &u8) -> Self {
        let _ = self.add_char(c); // For initialization
        self
    }
}