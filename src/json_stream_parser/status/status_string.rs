use serde_json::Value;

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusTrait};

#[derive(Debug)]
pub(crate) struct StatusString {
    string_in_progress: Vec<u8>,
    is_escaping: bool,
}

impl StatusTrait for StatusString {
    fn new() -> Self {
        Self {
            string_in_progress: Vec::new(),
            is_escaping: false
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (self.is_escaping, c) {
            (false, b'"') => {
                // String end
                let mut out_vec = Vec::<u8>::new(); // To be swapped with current data, since that one is no longer needed after this return
                std::mem::swap(&mut self.string_in_progress, &mut out_vec);
                let final_string = String::from_utf8(out_vec)?;
                return Ok(Some((Some(Value::String(final_string)), Some(Status::Done(super::StatusDone::default())))))
            },
            (false, b'\\') => {
                self.is_escaping = true;
                return Ok(None);
            },
            (true, _) => {
                todo!()
            },
            _ => {
                self.string_in_progress.push(*c);
                return Ok(None);
            }
        }
    }

    fn flush(&mut self) -> Option<Vec<u8>> {
        if self.string_in_progress.len() > 0 {
            let mut new_bytes: Vec<u8> = Vec::new();
            std::mem::swap(&mut new_bytes, &mut self.string_in_progress);
            return Some(new_bytes)
        } else {
            return None;
        }
    }
    
    fn with_char(mut self, c: &u8) -> Self {
        let _ = self.add_char(c); // For initialization
        self
    }
}