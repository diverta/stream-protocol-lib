use serde_json::Value;
use status_array::StatusArray;
use status_bool::StatusBool;
use status_done::StatusDone;
use status_none::StatusNone;
use status_null::StatusNull;
use status_number::StatusNumber;
use status_object::StatusObject;
use status_string::StatusString;

use super::error::ParseError;

pub(crate) mod status_none;
pub(crate) mod status_array;
pub(crate) mod status_bool;
pub(crate) mod status_null;
pub(crate) mod status_number;
pub(crate) mod status_object;
pub(crate) mod status_string;
pub(crate) mod status_done;

#[derive(Debug, PartialEq)]
pub(crate) enum Status {
    None(StatusNone), // Typically at the very first iteration, when the root object is still undefined
    Null(StatusNull),
    Bool(StatusBool),
    Number(StatusNumber),
    String(StatusString),
    Array(StatusArray),
    Object(StatusObject),
    Done(StatusDone) // Current status completed => go up to parent
}

pub(crate) trait StatusTrait {
    fn new() -> Self;
    // Returns the potential data for this status. Only returns if a full row is ready, in which case the current item must be considered completed
    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError>;
    // Return the current buffer on demand. Each status must handle its inner buffer accordingly
    fn flush(&mut self) -> Option<Value>; // Returns partial buffer of this status as JsonValue. If string, it will be valid (partial bytes are kept)
    // Helper for initialization with character
    fn with_char(self, c: &u8) -> Self;
    // If a status has a lingering state to handle (such as updating the buffer), it can optionally implement the finishing processing
    fn finish(&mut self) -> Result<Option<Value>, ParseError> {
        Ok(None)
    }
}

/// A StatusTrait wrapper for each Status for convenience
impl StatusTrait for Status {
    fn new() -> Self {
        Status::None(StatusNone::new())
    }

    fn add_char(&mut self, c: &u8) -> Result<Option<(Option<Value>, Option<Status>)>, ParseError> {
        match self {
            Status::None(s) => s.add_char(c),
            Status::Null(s) => s.add_char(c),
            Status::Bool(s) => s.add_char(c),
            Status::Number(s) => s.add_char(c),
            Status::String(s) => s.add_char(c),
            Status::Array(s) => s.add_char(c),
            Status::Object(s) => s.add_char(c),
            Status::Done(_) => { unreachable!("Done status must not be final") }
        }
    }
    
    fn flush(&mut self) -> Option<Value> {
        match self {
            Status::None(s) => s.flush(),
            Status::Null(s) => s.flush(),
            Status::Bool(s) => s.flush(),
            Status::Number(s) => s.flush(),
            Status::String(s) => s.flush(),
            Status::Array(s) => s.flush(),
            Status::Object(s) => s.flush(),
            Status::Done(_) => { unreachable!("Done status must not be final") }
        }
    }
    
    fn with_char(mut self, c: &u8) -> Self {
        let _ = self.add_char(c); // For initialization
        self
    }

    fn finish(&mut self) -> Result<Option<Value>, ParseError> {
        match self {
            Status::None(s) => s.finish(),
            Status::Null(s) => s.finish(),
            Status::Bool(s) => s.finish(),
            Status::Number(s) => s.finish(),
            Status::String(s) => s.finish(),
            Status::Array(s) => s.finish(),
            Status::Object(s) => s.finish(),
            Status::Done(_) => { unreachable!("Done status must not be final") }
        }
    }
}