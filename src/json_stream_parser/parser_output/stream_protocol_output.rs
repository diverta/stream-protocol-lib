use std::rc::Rc;

use serde_json::{json, Value};

use crate::json_stream_parser::status::Status;
use super::{ParentNode, ParserOutputTrait};

/// Implementation of the custom streaming protocol used by KurocoEdge JsonStream
pub struct StreamProtocolOutput;

pub const STREAM_VAR_PREFIX: &'static str = "$ke$";
pub const OPERATOR_ASSIGN: &'static str = "=";
pub const OPERATOR_APPEND: &'static str = "+=";

impl ParserOutputTrait for StreamProtocolOutput {
    fn new() -> Self {
        Self {}
    }

    #[inline(always)]
    fn on_init(&self, current_node_idx: usize, next_status: Option<&Status>) -> Option<String> {
        match next_status {
            Some(Status::String(_)) => {
                Some(self.make_row(current_node_idx, OPERATOR_ASSIGN, r#""""#))
            }
            Some(Status::Array(_)) => {
                Some(self.make_row(current_node_idx, OPERATOR_ASSIGN, "[]"))
            },
            Some(Status::Object(_)) => {
                Some(self.make_row(current_node_idx, OPERATOR_ASSIGN, "{}"))
            },
            _ => None
        }
    }

    #[inline(always)]
    fn on_status_complete(
        &self,
        parent_status: &Status,
        current_status: &Status,
        current_node_idx: usize,
        output_value: Option<Rc<Value>>
    ) -> Option<String> {
        let operator = match (parent_status, current_status) {
            (Status::None(_), Status::Null(_) | Status::Bool(_) | Status::Number(_)) => OPERATOR_ASSIGN,
            (Status::None(_), _) => OPERATOR_APPEND,
            (Status::Object(_), _) => OPERATOR_APPEND,
            (Status::Array(_), _) => OPERATOR_APPEND,
            _ => unreachable!("Logic error : non covered status combination")
        };
        output_value.map(|output| {
            self.make_row(current_node_idx, operator, output.to_string())
        })
    }

    #[inline(always)]
    fn on_object_key_complete(
        &self,
        _key: &String
    ) -> Option<String> {
        None
    }

    #[inline(always)]
    fn on_flush(&self, current_node_idx: usize, flush_output: &Value) -> Option<String> {
        Some(self.make_row(current_node_idx, OPERATOR_APPEND, flush_output.to_string()))
    }

    #[inline(always)]
    fn on_new_subnode(
        &self,
        parent_node: ParentNode,
        current_status: &Status,
        parent_node_idx: usize,
        current_node_idx: usize,
    ) -> Option<String> {
        let init_row = match parent_node {
            ParentNode::Object(key) => {
                self.make_row(
                    parent_node_idx,
                    OPERATOR_APPEND,
                    json!({key: format!("{}{}", STREAM_VAR_PREFIX, current_node_idx)}).to_string(),
                )
            },
            ParentNode::Array(_key) => {
                self.make_row(
                    parent_node_idx,
                    OPERATOR_APPEND,
                    format!("\"{}{}\"", STREAM_VAR_PREFIX, current_node_idx),
                )
            },
        };
         
        match current_status {
            // For string, we need to initialize the row, as we will be appending parts
            Status::String(_) => {
                Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                    init_row,
                    self.make_row(
                        current_node_idx,
                        OPERATOR_ASSIGN,
                        "\"\"",
                    ),
                ))
            }
            Status::Object(_) => {
                Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                    init_row,
                    self.make_row(
                        current_node_idx,
                        OPERATOR_ASSIGN,
                        "{}",
                    ),
                ))
            }
            Status::Array(_) => {
                Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                    init_row,
                    self.make_row(
                        current_node_idx,
                        OPERATOR_ASSIGN,
                        "[]",
                    ),
                ))
            }
            Status::Null(_) | Status::Bool(_) | Status::Number(_) => None,
            _ => unreachable!("Status flow is invalid")
        }
    }
}

impl StreamProtocolOutput {
    #[inline(always)]
    fn make_row(&self, idx: usize, operator: &'static str, data: impl Into<String>) -> String {
        format!("{}{}{}\n", idx, operator, data.into())
    }
}