use std::{collections::HashMap, rc::Rc};
use crate::{json_key_path::JsonKeyPath, json_stream_parser::status::status_array::StatusArray, ref_index_generator::RefIndexGenerator, OPERATOR_APPEND, OPERATOR_ASSIGN, STREAM_VAR_PREFIX};

use node::{Node, NodeType};
use serde_json::{json, Map, Value};
use value_buffer::ValueBuffer;

use super::{error::ParseError, status::{status_none::StatusNone, status_object::{StatusObject, SubStatusObject}}, ParserEvent, Status, StatusTrait};

mod node;
mod value_buffer;

/// This mapper attempts to parse a byte stream as a JSON object
/// At the same time, it writes back using the KurocoEdge streaming protocol of partial JSON at appropriate times
/// Flush feature may be used to write the row in progress - otherwise a string writes only when it is completed
/// 
/// Note: this makes most straighforward use of the protocol
pub(crate) struct PartialJsonProtocolMapper<F> {
    key_path: JsonKeyPath,
    ref_index_generator: RefIndexGenerator,
    node_map: HashMap<usize, Node>,
    current_node_idx: usize,
    current_status: Status,
    event_map: HashMap<ParserEvent, HashMap<String,Vec<F>>>,
    is_done: bool,
    string_value_buffer: String, // Storing the string buffer that persists across flushes. Used by events
    value_buffer: Option<ValueBuffer>,
}

impl<F> PartialJsonProtocolMapper<F>
where F: Fn(Option<Rc<Value>>) -> ()
{
    pub(crate) fn new(
        ref_index_generator: RefIndexGenerator,
        current_node_idx: usize,
        enable_buffering: bool
    ) -> Self {
        let value_buffer = if enable_buffering {
            Some(ValueBuffer::new(json!({})))
        } else {
            None
        };
        Self {
            key_path: JsonKeyPath::new(),
            ref_index_generator,
            node_map: HashMap::new(),
            current_status: Status::None(StatusNone {}),
            current_node_idx,
            event_map: HashMap::new(),
            is_done: false,
            string_value_buffer: String::new(),
            value_buffer
        }
    }

    fn on_event_move_down(&mut self, key: &str) {
        self.key_path.move_down_object_or_array(key);
        // Register element begin events
        if let Some(list_maps_for_event) = self.event_map.get(&ParserEvent::OnElementBegin) {
            for event_key in list_maps_for_event.keys() {
                if self.key_path.match_expr(event_key) {
                    let event_fns = list_maps_for_event.get(event_key).unwrap();
                    for event_fn in event_fns {
                        event_fn(None);
                    }
                }
            }
        }
        // If string, clear buffer
        match &self.current_status {
            Status::String(_) => {
                if self.string_value_buffer.len() > 0 {
                    self.string_value_buffer.clear();
                }
            }
            _ => {}
        }
        if let Some(value_buffer) = self.value_buffer.as_mut() {
            value_buffer.pointer_down(key).unwrap(); // Panic here represents a logical error : if identified, to be fixed
            match &self.current_status {
                // We should also init object/array in case of nesting
                Status::Array(_) => {
                    if let Some(value_buffer) = self.value_buffer.as_mut() {
                        (*value_buffer).insert_at_pointer(Value::Array(Vec::new())).unwrap(); // If this panics then it is a logic error
                    }
                },
                Status::Object(_) => {
                    if let Some(value_buffer) = self.value_buffer.as_mut() {
                        (*value_buffer).insert_at_pointer(Value::Object(Map::new())).unwrap(); // If this panics then it is a logic error
                    }
                },
                _ => {}
            }
        }
    }

    fn on_event_move_up(&mut self, value: Option<Rc<Value>>) {
        if let Some(list_maps_for_event) = self.event_map.get(&ParserEvent::OnElementEnd) {
            for event_key in list_maps_for_event.keys() {
                if self.key_path.match_expr(event_key) {
                    let event_fns = list_maps_for_event.get(event_key).unwrap();
                    for event_fn in event_fns {
                        // If string, use the buffer
                        // Since we don't store previous node data, we can use the buffer to check whether we have been buffering a string
                        if self.string_value_buffer.len() > 0 {
                            // No choice but to clone the buffer if we want to support having several references to the same element
                            let string_value = self.string_value_buffer.clone();
                            event_fn(Some(Rc::new(Value::String(string_value))));
                        } else {
                            event_fn(value.as_ref().map(|v| Rc::clone(&v)));
                        }
                    }
                }
            }
        }
        self.string_value_buffer.clear();
        self.key_path.move_up();
        if let Some(value_buffer) = self.value_buffer.as_mut() {
            value_buffer.pointer_up();
        }
    }

    fn on_event_value_completed(&mut self, output_value: Option<Rc<Value>>) {
        if let Some(value_buffer) = self.value_buffer.as_mut() {
            if let Some(output_value) = output_value {
                let output_value_copy = output_value.as_ref().clone();
                match self.current_status {
                    Status::String(_) => {
                        // In case of String, we can't trust output_value, because any potential flushing removes data from it
                        // We need to use string_value_buffer in this case
                        (*value_buffer).insert_at_pointer(Value::String(self.string_value_buffer.clone())).unwrap(); // If this panics then it is a logic error
                    },
                    _ => {
                        (*value_buffer).insert_at_pointer(output_value_copy).unwrap(); // If this panics then it is a logic error
                    }
                }
            }
        }
    }
    
    pub fn get_buffered_data(&self) -> Option<&Value> {
        self.value_buffer.as_ref().map(|value_buffer| &value_buffer.root)
    }
    
    pub fn take_buffered_data(&mut self) -> Option<Value> {
        self.value_buffer.as_mut().map(|value_buffer| value_buffer.take_buffered_data())
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
        let add_char_to_status_result = self.current_status.add_char(c)?;
        if add_char_to_status_result.is_none()  {
            // Current status has absorbed the character and is maintained, no outside status change
            return Ok(None);
        }
        let (output_value, next_status) = add_char_to_status_result.unwrap();
        let output_value = output_value.map(|v| Rc::new(v));

        // Processing the result of the add_char based on the current status
        match (&mut self.current_status, next_status) {

            (Status::None(_), next_status) => {
                // Initialization completed : save
                let new_node_type = match next_status {
                    Some(Status::Object(_)) => {
                        NodeType::Object(None)
                    },
                    Some(Status::Array(_)) => {
                        NodeType::Array(0)
                    },
                    _ => NodeType::Basic
                };
                self.node_map.insert(self.current_node_idx, Node::new(None, new_node_type));
                let row: Option<String> = output_value
                    .as_ref()
                    .map(|output|
                        self.make_row(self.current_node_idx, OPERATOR_ASSIGN, output.to_string())
                    );
                self.current_status = next_status.unwrap(); // StatusNone always returns next status, switch to it whatever it is
                if let Some(value_buffer) = self.value_buffer.as_mut() {
                    if let Some(output_value_ref) = output_value.as_ref() {
                        let output_value_copy = output_value_ref.as_ref().clone();
                        (*value_buffer).insert_at_pointer(output_value_copy).unwrap(); // Update root, as pointer should have not been moved yet
                    }
                }
                return Ok(row);
            },

            // A status has been completed
            (current_status, Some(Status::Done(status_done))) => {
                let current_node = self.node_map.get(&self.current_node_idx);
                if let Some(Value::String(val)) = output_value.as_deref() {
                    // Push output string into buffer before any potential self.on_event_move_up
                    self.string_value_buffer.push_str(val);
                }
                if current_node.is_none() {
                    // Parent object finished
                    self.is_done = true;
                    // self.on_event_move_up(output_value.as_ref()); Needed ?
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
                    self.on_event_value_completed(output_value.as_ref().map(|val| Rc::clone(&val)));
                    self.on_event_move_up(output_value.as_ref().map(|v| Rc::clone(&v)));
                    return Ok(output_value.map(|output| {
                        self.make_row(self.current_node_idx, operator, output.to_string())
                    }));
                }
                let current_idx = self.current_node_idx;
                let parent_idx = current_node.parent_idx.unwrap();
                // Already update current_node_idx cursor back to the parent value.
                // For clarity, let's not use it afterwards (use parent_idx instead)
                self.current_node_idx = parent_idx;

                let parent_node = self.node_map.get_mut(&parent_idx);
                if parent_node.is_none() {
                    // The parent node (which is now current) doesn't exist : we are done
                    // This should only happen for a top level basic json type (string, number, null, bool)
                    self.on_event_value_completed(output_value.as_ref().map(|val| Rc::clone(&val)));
                    self.on_event_move_up(output_value.as_ref().map(|v| Rc::clone(&v)));
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
                                if potential_key.is_some() {
                                    // The key exists => we are returning from the object value
                                    let key = potential_key.take().unwrap(); // Get the key, emptying the node's parameter
                                    let row = match current_status {
                                        // The way to write the row, however, depends on the type
                                        // Basic types, we have to append to the parent object itself
                                        Status::Null(_)|
                                        Status::Bool(_) |
                                        Status::Number(_) => {
                                            let value = output_value.as_ref().unwrap(); // A basic type, when Done, absolutely returns a value
                                            Some(self.make_row(
                                                parent_idx,
                                                OPERATOR_APPEND,
                                                json!({key: value}).to_string()
                                            ))
                                        },
                                        // For strings, we have already initialized it, so append to self
                                        Status::String(_) => {
                                            if let Some(value) = output_value.as_ref() {
                                                if value.as_str().unwrap().len() > 0 {
                                                    Some(self.make_row(
                                                        current_idx,
                                                        OPERATOR_APPEND,
                                                        value.to_string()
                                                    ))
                                                } else {
                                                    None
                                                }
                                            } else {
                                                // Value might not be present if flushed
                                                None
                                            }
                                        },
                                        _ => unreachable!("All base types are covered, aren't they?")
                                    };
                                    self.on_event_value_completed(output_value.as_ref().map(|val| Rc::clone(&val)));
                                    self.on_event_move_up(output_value);
                                    self.current_status = Status::Object(StatusObject {
                                        substatus: SubStatusObject::BeforeKV(false)
                                    });
                                    if status_done.done_object {
                                        // Not only the value is completed, but the current object must be too : go back up once again
                                        self.move_up();
                                    }
                                    return Ok(row);
                                } else {
                                    // Key does not exist yet => we are returning from the String value for the key : save it and continue
                                    let value = output_value.unwrap(); // String value for the object key must exist
                                    *potential_key = Some(value.as_str().unwrap().to_string());
                                    self.current_status = Status::Object(StatusObject {
                                        substatus: SubStatusObject::BetweenKV(false)
                                    });
                                    return Ok(None);
                                }
                            },
                            node::NodeType::Array(_) => {
                                let row = match current_status {
                                    // The way to write the row, however, depends on the type
                                    // Basic types, we have to append to the parent object itself
                                    Status::Null(_)|
                                    Status::Bool(_) |
                                    Status::Number(_) => {
                                        let value = output_value.as_ref().unwrap();
                                        Some(self.make_row(
                                            parent_idx,
                                            OPERATOR_APPEND,
                                            value.to_string()
                                        ))
                                    },
                                    // For strings, we have already initialized it, so append to self
                                    Status::String(_) => {
                                        if let Some(value) = output_value.as_ref() {
                                            if value.as_str().unwrap().len() > 0 {
                                                Some(self.make_row(
                                                    current_idx,
                                                    OPERATOR_APPEND,
                                                    value.to_string()
                                                ))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    },
                                    _ => unreachable!("All base types are covered, aren't they?")
                                };
                                self.on_event_value_completed(output_value.as_ref().map(|val| Rc::clone(&val)));
                                self.on_event_move_up(output_value);
                                self.current_status = Status::Array(StatusArray { comma_matched: status_done.comma_matched });
                                if status_done.done_array {
                                    // Not only the value is completed, but the current array must be too
                                    // => go back up once again
                                    self.move_up();
                                }
                                return Ok(row);
                            },
                            node::NodeType::Basic => {
                                unreachable!("Nested data cannot return into non-object or non-array")
                            },
                        }
                    },
                    Status::Object(_) | Status::Array(_) => {
                        // Returning from a nested object or array => the data append has already happened, so just move up
                        match &mut parent_node.node_type {
                            NodeType::Object(ref mut potential_key) => {
                                *potential_key = None; // On return from a nested item, make sure the object key is unset
                                self.on_event_value_completed(output_value.as_ref().map(|val| Rc::clone(&val)));
                                self.on_event_move_up(None);
                                self.current_status = Status::Object(StatusObject {
                                    substatus: SubStatusObject::BeforeKV(status_done.comma_matched)
                                })
                            },
                            NodeType::Array(_) => {
                                self.on_event_value_completed(output_value.as_ref().map(|val| Rc::clone(&val)));
                                self.on_event_move_up(None);
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
                // Handling String values for Object & Array
                _current_status @ (Status::Object(_) | Status::Array(_)),
                Some(new_status @ (
                    Status::String(_) |
                    Status::Null(_) |
                    Status::Bool(_) |
                    Status::Number(_)
                ))
            ) => {
                // Going down the tree into a basic or String type
                let new_node_idx = self.ref_index_generator.generate();
                self.node_map.insert(new_node_idx, Node::new(Some(self.current_node_idx), NodeType::Basic));
                let parent_node_idx = self.current_node_idx;
                self.current_node_idx = new_node_idx;
                self.current_status = new_status; // Become the new type
                let parent_node = self.node_map.get_mut(&parent_node_idx).unwrap();
                match &mut parent_node.node_type {
                    NodeType::Object(ref potential_key) => { // Only when we are parsing the string that is the value of the object (because key is existing)
                        if let Some(key) = potential_key {
                            let key_copy = key.clone(); // To fix borrowing issue
                            let result = match self.current_status {
                                // For string, we need to initialize the row, as we will be appending parts
                                Status::String(_) => {
                                    Ok(Some(format!("{}{}", // Double initialization : a new index, and a new string at that index
                                    self.make_row(
                                        parent_node_idx,
                                        OPERATOR_APPEND,
                                        json!({&key_copy: format!("{}{}", STREAM_VAR_PREFIX, new_node_idx)}).to_string(),
                                    ),
                                    self.make_row(
                                        self.current_node_idx,
                                        OPERATOR_ASSIGN,
                                        "\"\"",
                                    ),
                                )))
                                }
                                Status::Null(_) | Status::Bool(_) | Status::Number(_) => Ok(None),
                                _ => unreachable!("Status flow is invalid")
                            };
                            self.on_event_move_down(&key_copy);
                            return result;
                        } else {
                            // This String is being used to parse an object's key : do not write now, wait for the value
                            return Ok(None);
                        }
                    }
                    NodeType::Array(arr_idx) => { // Or when its the value of the array
                        let arr_idx_str = arr_idx.to_string(); // Converting the current array index (it has not been used yet)
                        *arr_idx = *arr_idx + 1;
                        let result = match self.current_status {
                            // For string, we need to initialize the row, as we will be appending parts
                            Status::String(_) => {
                                Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
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
                                )))
                            }
                            Status::Null(_) | Status::Bool(_) | Status::Number(_) => Ok(None),
                            _ => unreachable!("Status flow is invalid")
                        };
                        self.on_event_move_down(&arr_idx_str);
                        return result;
                    },
                    _ => unreachable!("Logic error : String cannot be a child of non-object and non-array")
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
                let parent_node = self.node_map.get_mut(&parent_node_idx).unwrap();
                match &mut parent_node.node_type {
                    NodeType::Object(Some(key)) => {
                        let key_copy = key.clone(); // To fix borrowing issue
                        let result = Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                json!({&key_copy: format!("{}{}", STREAM_VAR_PREFIX, new_node_idx)}).to_string(),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "{}",
                            ),
                        )));
                        self.on_event_move_down(&key_copy);
                        return result;
                    },
                    NodeType::Array(arr_idx) => {
                        let arr_idx_str = arr_idx.to_string(); // Converting the current array index (it has not been used yet)
                        *arr_idx = *arr_idx + 1;
                        let result = Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
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
                        self.on_event_move_down(&arr_idx_str);
                        return result;
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
                self.node_map.insert(new_node_idx, Node::new(Some(self.current_node_idx), NodeType::Array(0)));
                let parent_node_idx = self.current_node_idx;
                self.current_node_idx = new_node_idx;
                self.current_status = new_status;
                let parent_node = self.node_map.get_mut(&parent_node_idx).unwrap();
                match &mut parent_node.node_type {
                    NodeType::Object(Some(key)) => {
                        let key_copy = key.clone(); // To fix borrowing issue
                        let result = Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
                            self.make_row(
                                parent_node_idx,
                                OPERATOR_APPEND,
                                json!({&key_copy: format!("{}{}", STREAM_VAR_PREFIX, new_node_idx)}).to_string(),
                            ),
                            self.make_row(
                                self.current_node_idx,
                                OPERATOR_ASSIGN,
                                "[]",
                            ),
                        )));
                        self.on_event_move_down(&key_copy);
                        return result;
                    },
                    NodeType::Array(arr_idx) => {
                        let arr_idx_str = arr_idx.to_string(); // Converting the current array index (it has not been used yet)
                        *arr_idx = *arr_idx + 1;
                        let result = Ok(Some(format!("{}{}", // Double initialization : a new index, and a new object at that index
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
                        self.on_event_move_down(&arr_idx_str);
                        return result;
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

    #[inline]
    pub fn flush(&mut self) -> Option<String> {
        match self.current_status.flush() {
            Some(data) => {
                match &data {
                    Value::String(str) => {
                        // Save in buffer
                        self.string_value_buffer.push_str(str);
                    },
                    _ => {}
                }
                Some(format!("{}{}{}\n", self.current_node_idx, OPERATOR_APPEND, data.to_string()))
            }
            None => None,
        }
    }

    // Silently move up the node map
    #[inline]
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
                        NodeType::Array(_1) => {
                            self.current_status = Status::Array(StatusArray { comma_matched: false }); 
                        },
                        NodeType::Basic => unreachable!("Nested data cannot return into non-object or non-array"),
                    }
                }
            }
        }
        self.on_event_move_up(None);
    }

    /// Attach a function to be executed when an event occurs at a given element
    /// Element is a simple string path to a JSON key. Ex: "parent.child.grandchildren[0].name"
    /// Json key path uses dot notation to separate levels. Array index can be replaced with wildcard (*) to match every element
    pub fn add_event_handler(&mut self, event: ParserEvent, element: String, func: F) {
        let list_maps_for_event = self
            .event_map
            .entry(event)
            .or_insert(HashMap::new()
        );
        let event_list = list_maps_for_event
            .entry(element)
            .or_insert(Vec::new());
        event_list.push(func);
    }
}