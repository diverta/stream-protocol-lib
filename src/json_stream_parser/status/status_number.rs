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
        match c {
            b'0'..=b'9' | b'e' | b'+' | b'-' | b'.'  // Listing all possible number characters, it's fine if the sequence is incorrect - serde will validate it
            => {
                self.match_so_far.push(*c);
                return Ok(None);
            }
            _ => {
                let mut out_vec = Vec::<u8>::new(); // To be swapped with current data, since that one is no longer needed after this return
                std::mem::swap(&mut self.match_so_far, &mut out_vec);
                match serde_json::from_slice::<Number>(&out_vec) {
                    Ok(serde_number) => {
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
                    },
                    Err(err) => {
                        return Err(ParseError::new(format!("Invalid number : {err}")));
                    }
                }
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
    
    fn finish(&mut self) -> Result<Option<Value>, ParseError> {
        if self.match_so_far.len() > 0 {
            let mut out_vec = Vec::<u8>::new(); // To be swapped with current data, since that one is no longer needed after this return
            std::mem::swap(&mut self.match_so_far, &mut out_vec);
            serde_json::from_slice::<Number>(&out_vec)
                .map(|n| Some(n.into()))
                .map_err(|_|
                    ParseError::new(
                        format!("Error parsing serde number on finish : {}",
                            String::from_utf8_lossy(&self.match_so_far)
                        )
                    )
                )
        } else {
            Ok(None)
        }
    }
}