use serde_json::Value;

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusTrait};

#[derive(Debug)]
pub(crate) struct StatusString {
    string_in_progress: Vec<u8>,
    is_escaping: bool,
    is_object_key: bool,
}

impl StatusString {
    pub fn with_object_key(mut self) -> Self {
        self.is_object_key = true;
        self
    }
}

impl StatusTrait for StatusString {
    fn new() -> Self {
        Self {
            string_in_progress: Vec::new(),
            is_escaping: false,
            is_object_key: false
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (self.is_escaping, c) {
            (false, b'"') => {
                // String end
                if self.string_in_progress.len() > 0 {
                    let mut out_vec = Vec::<u8>::new(); // To be swapped with current data, since that one is no longer needed after this return
                    std::mem::swap(&mut self.string_in_progress, &mut out_vec);
                    let final_string = String::from_utf8(out_vec)?;
                    return Ok(Some((Some(Value::String(final_string)), Some(Status::Done(super::StatusDone::default())))))
                } else {
                    // Empty string : may happen if a flush has occured right before the end
                    return Ok(Some((None, Some(Status::Done(super::StatusDone::default())))))
                }
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

    fn flush(&mut self) -> Option<String> {
        if !self.is_object_key && self.string_in_progress.len() > 0 {
            let output: String = match std::str::from_utf8(&mut self.string_in_progress) {
                Ok(data) => {
                    // Partial bytes are valid string, lucky
                    let output: String = data.to_string();
                    self.string_in_progress.clear();
                    output
                },
                Err(e) => {
                    // Drain the UTF8-valid part of the buffer, leaving the remaining not-yet-parseable bytes
                    let valid_prefix: Vec<u8> = self.string_in_progress.drain(..e.valid_up_to()).collect();
                    String::from_utf8(valid_prefix).unwrap()
                }
            };
            if output.len() > 0 {
                Some(Value::String(output).to_string()) // Converting into JSON String
            } else {
                None
            }
        } else {
            return None;
        }
    }
    
    fn with_char(mut self, c: &u8) -> Self {
        let _ = self.add_char(c); // For initialization
        self
    }
}