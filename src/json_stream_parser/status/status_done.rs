
#[derive(Debug, Default, PartialEq)]
/// Struct needed to store a couple of params along with being Done parsing the subitem
pub struct StatusDone {
    pub done_object: bool, // True if done on detecting '}' as inner value stop condition (currently only needed for Number) to double up
    pub done_array: bool, // True if done on detecting ']' as inner value stop condition (currently only needed for Number) to double up
    pub comma_matched: bool, // True if done on detecting ',' as inner value stop condition (currently only needed for Number) to double up
}

impl StatusDone {
    pub fn new(done_object: bool, done_array: bool, comma_matched: bool) -> Self {
        return Self {
            done_object,
            done_array,
            comma_matched
        }
    }
}