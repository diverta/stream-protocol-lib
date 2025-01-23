use serde_json::{Number, Value};

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusTrait};

#[derive(Debug, PartialEq)]
pub(crate) struct StatusNumber {
    dot_matched: bool,
    match_so_far: Vec<u8> // Contains incomplete sequence
}

impl StatusTrait for StatusNumber {
    fn new() -> Self {
        Self {
            dot_matched: false,
            match_so_far: Vec::new()
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (&self.match_so_far[..], self.dot_matched, c) {
            (_, _, b'0'..=b'9') | // Digits are OK anytime
            ([], false, b'-') | // Negative sign can only be the first character
            ([b'-', b'0'..=b'9', ..] | [b'0'..=b'9',..], false, b'.') // The dot may only follow at least one digit with optional sign, if not already matched
            => {
                if c == &b'.' {
                    self.dot_matched = true;
                }
                self.match_so_far.push(*c);
                return Ok(None);
            }
            ([_, ..], _, c) // At least one character mapped - anything non number will be considered a return
            => {
                let mut out_vec = Vec::<u8>::new(); // To be swapped with current data, since that one is no longer needed after this return
                std::mem::swap(&mut self.match_so_far, &mut out_vec);
                let serde_number = if self.dot_matched{
                    let number: f64 = String::from_utf8(out_vec)?.parse()?;
                    Number::from_f64(number)
                        .ok_or(ParseError::new(format!("Float is invalid : {}", number)))?
                } else {
                    let number: i128 = String::from_utf8(out_vec)?.parse()?;
                    Number::from_i128(number)
                        .ok_or(ParseError::new(format!("Integer is invalid : {}", number)))?
                };
                // Number is a bit special base type in that we don't know whether we are done parsing until we hit an invalid character
                // This makes it that we might not only be done with the number itself, but also with the parent object or array
                // if hitting '}' or ']' respectively. So we have to pass the flag to the parent above
                let mut done_object = false;
                let mut done_array = false;
                let mut comma_matched = false;
                match c {
                    b'}' => {
                        done_object = true;
                    },
                    b']' => {
                        done_array = true;
                    },
                    b',' => {
                        comma_matched = true;
                    },
                    _ => {}
                }
                return Ok(Some((Some(Value::Number(serde_number)), Some(Status::Done(super::StatusDone::new(
                    done_object,
                    done_array,
                    comma_matched
                ))))))
            }
            _ => {
                return Err(ParseError::new("Invalid number"))
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