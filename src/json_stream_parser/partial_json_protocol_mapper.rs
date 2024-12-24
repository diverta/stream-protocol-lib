use std::collections::HashMap;
use crate::{json_stream_parser::status::status_array::StatusArray, ref_index_generator::RefIndexGenerator, OPERATOR_APPEND, OPERATOR_ASSIGN, STREAM_VAR_PREFIX};

use node::{Node, NodeType};
use pin_project::pin_project;
use serde_json::json;

use super::{error::ParseError, status::{status_none::StatusNone, status_object::{StatusObject, SubStatusObject}}, Status, StatusTrait};

mod node;

/// This mapper attempts to parse a byte stream as a JSON object
/// At the same time, it writes back using the KurocoEdge streaming protocol of partial JSON at appropriate times
/// Flush feature may be used to write the row in progress - otherwise a string writes only when it is completed
/// 
/// Note: this makes most straighforward use of the protocol
#[pin_project]
pub(crate) struct PartialJsonProtocolMapper {
    ref_index_generator: RefIndexGenerator,
    node_map: HashMap<usize, Node>,
    current_node_idx: usize,
    current_status: Status,
    is_done: bool
}

impl PartialJsonProtocolMapper {
    pub(crate) fn new(ref_index_generator: RefIndexGenerator, current_node_idx: usize) -> Self {
        Self {
            ref_index_generator,
            node_map: HashMap::new(),
            current_status: Status::None(StatusNone {}),
            current_node_idx,
            is_done: false
        }
    }

    #[inline]
    fn make_row(&mut self, idx: usize, operator: &'static str, data: impl Into<String>) -> String {
        format!("{}{}{}\n", idx, operator, data.into())
    }

    #[inline]
    pub(crate) fn add_char(&mut self, c: &u8) -> Result<Option<impl Into<String>>, ParseError> {
        if self.is_done {
            return Ok(None);
        }
        //println!("ADD '{}'at idx={} {:?}", byte_to_char(c), self.current_node_idx, self.current_status);
        let add_char_to_status_result = self.current_status.add_char(c)?;
        if add_char_to_status_result.is_none()  {
            // Current status has absorbed the character and is maintained, no outside status change
            return Ok(None);
        }
        let (output_value, next_status) = add_char_to_status_result.unwrap();

        // Processing the result of the add_char based on the current status
        match (&mut self.current_status, next_status) {

            (Status::None(_), next_status) => {
                // Initialization completed : save
                let new_node_type = match next_status {
                    Some(Status::Object(_)) => {
                        NodeType::Object(None)
                    },
                    Some(Status::Array(_)) => {
                        NodeType::Array
                    },
                    _ => NodeType::Basic
                };
                self.node_map.insert(self.current_node_idx, Node::new(None, new_node_type));
                let row: Option<String> = output_value
                    .map(|output|
                        self.make_row(self.current_node_idx, OPERATOR_ASSIGN, output.to_string())
                    );
                self.current_status = next_status.unwrap(); // StatusNone always returns next status, switch to it whatever it is
                return Ok(row);
            },

            // A status has been completed
            (current_status, Some(Status::Done(status_done))) => {
                let current_node = self.node_map.get(&self.current_node_idx);
                if current_node.is_none() {
                    // Parent object finished
                    self.is_done = true;
                    return Ok(None);
                };
                let current_node = current_node.unwrap();
                if current_node.parent_idx.is_none() {
                    // No parent within node : parsing done, however the output must be appended
                    self.is_done = true;
                    let operator = match current_status {
                        // For the simple types, the output is not returned at first match
                        // So the init of the top level might never happen
                        Status::Null(_) | Status::Bool(_) | Status::Number(_) => OPERATOR_ASSIGN,
                        _ => OPERATOR_APPEND
                    };
                    return Ok(output_value.map(|output| {
                        self.make_row(self.current_node_idx, operator, output.to_string())
                    }));
                }
                let parent_idx = current_node.parent_idx.unwrap();
                // Already update current_node_idx cursor back to the parent value.
                // For clarity, let's not use it afterwards (use parent_idx instead)
                self.current_node_idx = parent_idx;

                let parent_node = self.node_map.get_mut(&parent_idx);
                if parent_node.is_none() {
                    // The parent node (which is now current) doesn't exist : we are done
                    // This should only happen for a top level basic json type (string, number, null, bool)
                    return Ok(
                        output_value.map(|output|
                            self.make_row(
                                parent_idx,
                                OPERATOR_ASSIGN, 
                                output.to_string()
                            )
                        )
                    )
                }
                let parent_node = parent_node.unwrap();
                match current_status {
                    Status::Null(_)|
                    Status::Bool(_) |
                    Status::Number(_) |
                    Status::String(_)
                    => {
                        match &mut parent_node.node_type {
                            node::NodeType::Object(ref mut potential_key) => {
                                let value = output_value.unwrap(); // A basic type, when Done, absolutely returns a value
                                if potential_key.is_some() {
                                    // The key exists => we are returning from specifically String value
                                    let key = potential_key.take().unwrap(); // Get the key, emptying the node's parameter
                                    let row = self.make_row(
                                        parent_idx,
                                        OPERATOR_APPEND,
                                        json!({key: value}).to_string()
                                    );
                                    println!("APPENDING STRING {value}");
                                    self.current_status = Status::Object(StatusObject {
                                        substatus: SubStatusObject::BeforeKV(false)
                                    });
                                    if status_done.done_object {
                                        // Not only the value is completed, but the current object must be too : go back up once again
                                        self.move_up();
                                    }
                                    return Ok(Some(row));
                                } else {
                                    // Key does not exist yet => we are returning from the key : save it and continue
                                    *potential_key = Some(value.as_str().unwrap().to_string());
                                    self.current_status = Status::Object(StatusObject {
                                        substatus: SubStatusObject::BetweenKV(false)
                                    });
                                    return Ok(None);
                                }
                            },
                            node::NodeType::Array => {
                                self.current_status = Status::Array(StatusArray { comma_matched: status_done.comma_matched });
                                let output_value = output_value.unwrap();
                                let row = self.make_row(
                                    parent_idx,
                                    OPERATOR_APPEND,
                                    output_value.to_string()
                                );
                                if status_done.done_array {
                                    // Not only the value is completed, but the current array must be too
                                    // => go back up once again
                                    self.move_up();
                                }
                                return Ok(Some(row));
                            },
                            node::NodeType::Basic => {
                                unreachable!("Nested data cannot return into non-object or non-array")
                            },
                        }
                    },
                    Status::Object(_) | Status::Array(_) => {
                        // Returning from a nested object or array => the data append has already happened, so just move up
                        match &mut parent_node.node_type {
                            NodeType::Object(_c) => {
                                self.current_status = Status::Object(StatusObject {
                                    substatus: SubStatusObject::BeforeKV(status_done.comma_matched)
                                })
                            },
                            NodeType::Array => {
                                self.current_status = Status::Array(StatusArray {
                                    comma_matched: status_done.comma_matched
                                });
                            },
                            node::NodeType::Basic => {
                                unreachable!("Nested data cannot return into non-object or non-array")
                            },
                        }
                        return Ok(None)
                    },
                    _ => unreachable!("This status should not be possible at this time")
                }
            },

            (
                // Handling children values of Object & Array for simple types
                _current_status @ (Status::Object(_) | Status::Array(_)),
                Some(new_status @ (
                    Status::Null(_) |
                    Status::Bool(_) |
                    Status::Number(_)
                ))
            ) => {
                // Going down the tree into a basic type
                let new_node_idx = self.ref_index_generator.generate();
                self.node_map.insert(new_node_idx, Node::new(Some(self.current_node_idx), NodeType::Basic));
                self.current_node_idx = new_node_idx;
                self.current_status = new_status; // Become the new type
                return Ok(None);
            },

            (
                // Handling children values of Object & Array for Strings
                _current_status @ (Status::Object(_) | Status::Array(_)),
                Some(new_status @ Status::String(_))
            ) => {
                // Going down the tree into a basic type
                let new_node_idx = self.ref_index_generator.generate();
                self.node_map.insert(new_node_idx, Node::new(Some(self.current_node_idx), NodeType::Basic));
                let parent_node_idx = self.current_node_idx;
                self.current_node_idx = new_node_idx;
                self.current_status = new_status; // Become the new type
                let parent_node = self.node_map.get(&parent_node_idx).unwrap();
                match &parent_node.node_type {
                    NodeType::Object(Some(key)) => { // Only when we are parsing the string that is the value of the object (because key is existing)
                        return Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                json!({key: format!("{}{}", STREAM_VAR_PREFIX, new_node_idx)}).to_string(),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "\"\"",
                            ),
                        )));
                    }
                    NodeType::Array => { // Or when its the value of the array
                        return Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                format!("\"{}{}\"", STREAM_VAR_PREFIX, new_node_idx),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "\"\"",
                            ),
                        )));
                    },
                    _ => {
                        return Ok(None); // Handles the case of String as object key
                    }
                }
            },

            (
                // Nesting for objects
                _current_status @ (Status::Object(_) | Status::Array(_)),
                Some(new_status @ Status::Object(_))
            ) => {
                // Going down the tree into a new string : generate new index
                let new_node_idx = self.ref_index_generator.generate();
                self.node_map.insert(new_node_idx, Node::new(Some(self.current_node_idx), NodeType::Object(None)));
                let parent_node_idx = self.current_node_idx;
                self.current_node_idx = new_node_idx;
                self.current_status = new_status;
                let parent_node = self.node_map.get(&parent_node_idx).unwrap();
                match &parent_node.node_type {
                    NodeType::Object(Some(key)) => {
                        return Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                json!({key: format!("{}{}", STREAM_VAR_PREFIX, new_node_idx)}).to_string(),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "{}",
                            ),
                        )));
                    },
                    NodeType::Array => {
                        return Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                format!("\"{}{}\"", STREAM_VAR_PREFIX, new_node_idx),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "{}",
                            ),
                        )));
                    },
                    _ => unreachable!("Logic error : nesting is being made from an object which doesn't have its key")
                }
            },

            (
                // Nesting for arrays
                _current_status @ (Status::Object(_) | Status::Array(_)),
                Some(new_status @ Status::Array(_))
            ) => {
                // Going down the tree into a new string : generate new index
                let new_node_idx = self.ref_index_generator.generate();
                self.node_map.insert(new_node_idx, Node::new(Some(self.current_node_idx), NodeType::Array));
                let parent_node_idx = self.current_node_idx;
                self.current_node_idx = new_node_idx;
                self.current_status = new_status;
                let parent_node = self.node_map.get(&parent_node_idx).unwrap();
                match &parent_node.node_type {
                    NodeType::Object(Some(key)) => {
                        return Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                json!({key: format!("{}{}", STREAM_VAR_PREFIX, new_node_idx)}).to_string(),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "[]",
                            ),
                        )));
                    },
                    NodeType::Array => {
                        return Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                format!("\"{}{}\"", STREAM_VAR_PREFIX, new_node_idx),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "[]",
                            ),
                        )));
                    },
                    _ => unreachable!("Logic error : nesting is being made from an object which doesn't have its key")
                }
            },

            (cur_status, next_status) => {
                // For other statuses : this message should never occur after 100% of the logic is done.
                // If this error appears, then that one case has not been handled
                unimplemented!("The set ({:?}, {:?}) is not implemented", cur_status, next_status);
            }
        }
    }

    pub fn flush(&mut self) -> Option<Vec<u8>> {
        match self.current_status.flush() {
            Some(status_dump) => {
                // Not using make_row to avoid conversion into string which may be an error in case of mid-utf8 character cut
                let mut out_bytes = format!("{}{}", self.current_node_idx, OPERATOR_APPEND).as_bytes().to_vec();
                out_bytes.extend(status_dump);
                out_bytes.push(b'\n');
                Some(out_bytes)
            },
            None => None
        }
    }

    // Silently move up the node map
    pub fn move_up(&mut self) {
        let current_node = self.node_map.get(&self.current_node_idx);
        if current_node.is_none() {
            // Current object is top level
            self.is_done = true;
        } else {
            let current_node = current_node.unwrap();
            if current_node.parent_idx.is_none() {
                // No parent within node : parsing done
                self.is_done = true;
            } else {
                let parent_idx = current_node.parent_idx.unwrap();
                self.current_node_idx = parent_idx;
                let parent_node = self.node_map.get_mut(&parent_idx);
                if parent_node.is_none() {
                    // I dont see a usecase for this, but just in case (instead of unwrap)
                    self.is_done = true;
                } else {
                    let parent_node = parent_node.unwrap();
                    match &parent_node.node_type {
                        NodeType::Object(_) => {
                            self.current_status = Status::Object(StatusObject {
                                substatus: SubStatusObject::BeforeKV(false)
                            });
                        },
                        NodeType::Array => {
                            self.current_status = Status::Array(StatusArray { comma_matched: false }); 
                        },
                        NodeType::Basic => unreachable!("Nested data cannot return into non-object or non-array"),
                    }
                }
            }
        }
    }
}