use std::rc::Rc;

use serde_json::Value;

use super::status::Status;

pub mod stream_protocol_output;
pub mod parser_output_none;

/// Configurable output for the parser, allowing to return a custom output at specific parser events
pub trait ParserOutputTrait {
    fn new() -> Self;

    /// Fires when the first character is read
    /// Status is the detected status of the value type identified by the first character
    /// Might not be defined yet (for ex in case of whitespace as first character)
    fn on_init(
        &self,
        current_node_idx: usize,
        next_status: Option<&Status>
    ) -> Option<String>;

    /// Trigger when a status has been completed
    fn on_status_complete(
        &self,
        parent_status: &Status,
        current_status: &Status,
        current_node_idx: usize,
        output_value: Option<Rc<Value>>
    ) -> Option<String>;

    /// Trigger when an object key has been parsed
    fn on_object_key_complete(
        &self,
        key: &String
    ) -> Option<String>;

    /// Trigger when flush has been requested
    fn on_flush(
        &self,
        current_node_idx: usize,
        flush_output: &Value
    ) -> Option<String>;

    /// Trigger when a new node has been added to an object or an array
    fn on_new_subnode(
        &self,
        parent_node: ParentNode,
        current_status: &Status,
        parent_node_idx: usize,
        current_node_idx: usize,
    ) -> Option<String>;
}

/// Helper enum to identify cases of an object or array node
/// The attribute is the key
pub enum ParentNode {
    Object(String),
    Array(usize)
}