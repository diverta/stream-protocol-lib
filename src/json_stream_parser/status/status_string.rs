use serde_json::Value;

use crate::json_stream_parser::error::ParseError;

use super::{Status, StatusTrait};

#[derive(Debug)]
pub(crate) struct StatusString {
    string_in_progress: Vec<u8>,
    escape: EscapeState,
    is_object_key: bool,
}

#[derive(Debug)]
pub(crate) enum EscapeState {
    None, // Not escaping
    Began, // After \ is detected
    U(String) // UTF8 digits (up to 4)
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
            escape: EscapeState::None,
            is_object_key: false
        }
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match (&mut self.escape, c) {
            (EscapeState::None, b'"') => {
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
            (EscapeState::None, b'\\') => {
                self.escape = EscapeState::Began;
                return Ok(None);
            },
            (EscapeState::Began, escaped_char) => {
                // Exhaustive escape pattern matching based on the page 5 of the following document :
                // https://ecma-international.org/wp-content/uploads/ECMA-404.pdf
                match escaped_char {
                    b'"' // quotation mark
                    | b'\\' // reverse solidus
                    | b'/' // solidus
                    => {
                        // Push the character as is
                        self.string_in_progress.push(*escaped_char);
                        self.escape = EscapeState::None;
                        return Ok(None);
                    }
                    b'b' // backspace
                    | b'f' // form feed
                    => {
                        // Silently absorb these
                        self.escape = EscapeState::None;
                        return Ok(None);
                    }
                    b'n' => { // newline
                        self.string_in_progress.push(b'\n');
                        self.escape = EscapeState::None;
                        return Ok(None);
                    }
                    b'r' => { // carriage return
                        self.string_in_progress.push(b'\r');
                        self.escape = EscapeState::None;
                        return Ok(None);
                    }
                    b't' => { // tab
                        self.string_in_progress.push(b'\t');
                        self.escape = EscapeState::None;
                        return Ok(None);
                    }
                    b'u' => { // Beginning of UTF8 capture
                        self.escape = EscapeState::U(String::with_capacity(4));
                        return Ok(None);
                    }
                    _ => return Err("Invalid JSON escaped character".into())
                }
            },
            (EscapeState::U(digits_so_far), escaped_char) => {
                match digits_so_far.len() {
                    0 | 1 | 2 => {
                        // Non-final character absorb
                        digits_so_far.push(*escaped_char as char);
                        return Ok(None);
                    },
                    3 => {
                        // Last digit capture
                        digits_so_far.push(*escaped_char as char);
                        let Some(utf8_char) = u32::from_str_radix(&digits_so_far, 16)
                            .ok()
                            .and_then(|input_base10| {
                                char::from_u32(input_base10)
                            })
                            else { return Err(format!("JSON contains invalid UTF digits : \\u{}", &digits_so_far).into()) };
                        self.string_in_progress.extend_from_slice(utf8_char.to_string().as_bytes());                    
                        self.escape = EscapeState::None;
                        return Ok(None);
                    },
                    _ => return Err("Escapted JSON UTF8 has detected more than 4 digits".into())
                }
            },
            (EscapeState::None, _) => {
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