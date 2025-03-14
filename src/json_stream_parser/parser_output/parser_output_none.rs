use std::rc::Rc;

use serde_json::Value;

use crate::json_stream_parser::status::Status;
use super::{ParentNode, ParserOutputTrait};

/// Implementation of parser with no output (for usage when only buffered data is needed)
pub struct ParserOutputNone;

impl ParserOutputTrait for ParserOutputNone {
    fn new() -> Self {
        Self {}
    }

    #[inline(always)]
    fn on_init(&self, _current_node_idx: usize, _next_status: Option<&Status>) -> Option<String> {
       None
    }

    #[inline(always)]
    fn on_status_complete(
        &self,
        _parent_status: &Status,
        _current_status: &Status,
        _current_node_idx: usize,
        _output_value: Option<Rc<Value>>
    ) -> Option<String> {
        None
    }

    #[inline(always)]
    fn on_object_key_complete(
        &self,
        _key: &String
    ) -> Option<String> {
        None
    }

    #[inline(always)]
    fn on_flush(&self, _current_node_idx: usize, _flush_output: &Value) -> Option<String> {
        None
    }

    #[inline(always)]
    fn on_new_subnode(
        &self,
        _parent_node: ParentNode,
        _current_status: &Status,
        _parent_node_idx: usize,
        _current_node_idx: usize,
    ) -> Option<String> {
        None
    }
}