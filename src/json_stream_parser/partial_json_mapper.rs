use std::{collections::HashMap, rc::Rc};
use crate::{json_key_path::JsonKeyPath, json_stream_parser::status::status_array::StatusArray, ref_index_generator::RefIndexGenerator};

use node::{Node, NodeType};
use serde_json::{json, Map, Value};
use value_buffer::ValueBuffer;

use super::{error::ParseError, parser_options::ParserOptions, parser_output::{ParentNode, ParserOutputTrait}, status::{status_none::StatusNone, status_object::{StatusObject, SubStatusObject}}, ParserEvent, Status, StatusTrait};

mod node;
mod value_buffer;

/// This mapper attempts to parse a byte stream as a JSON object
/// At the same time, it writes back using the KurocoEdge streaming protocol of partial JSON at appropriate times
/// Flush feature may be used to write the row in progress - otherwise a string writes only when it is completed
/// 
/// Note: this makes most straighforward use of the protocol
pub(crate) struct PartialJsonMapper<F, O> {
    key_path: JsonKeyPath,
    ref_index_generator: RefIndexGenerator,
    node_map: HashMap<usize, Node>,
    current_node_idx: usize,
    current_status: Status,
    event_map: HashMap<ParserEvent, HashMap<String,Vec<F>>>,
    is_done: bool,
    string_value_buffer: String, // Storing the string buffer that persists across flushes. Used by events
    value_buffer: Option<ValueBuffer>,
    parser_options: ParserOptions,
    parser_output: O,
}

impl<F, O> PartialJsonMapper<F, O>
where
    F: Fn(Option<Rc<Value>>) -> (),
    O: ParserOutputTrait
{
    pub(crate) fn new(
        ref_index_generator: RefIndexGenerator,
        current_node_idx: usize,
        enable_buffering: bool,
        parser_options: ParserOptions,
        parser_output: O
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
            value_buffer,
            parser_options,
            parser_output
        }
    }

    #[inline]
    fn is_ignoring_current_output(&self) -> bool {
        return self.node_map.get(&self.current_node_idx).map(|node| node.node_ignore_output).unwrap_or(false);
    }

    #[inline]
    fn is_ignoring_current_buffer(&self) -> bool {
        return self.node_map.get(&self.current_node_idx).map(|node| node.node_ignore_buffer).unwrap_or(false);
    }

    /// If return is false, the output should be ignored
    #[inline]
    fn on_event_move_down(&mut self, key: &str) {
        self.key_path.move_down_object_or_array(key);
        if !self.is_ignoring_current_output() {
            if let Some(current_node) = self.node_map.get_mut(&self.current_node_idx) {
                // If not ignoring still, confirm filters now
                if let Some(output_whitelist) = self.parser_options.filter.output_whitelist.as_ref() {
                    if !self.key_path.match_list(output_whitelist.iter().collect(), true) {
                        current_node.node_ignore_output = true;
                    }
                }
            }
        }
        if !self.is_ignoring_current_buffer() {
            if let Some(current_node) = self.node_map.get_mut(&self.current_node_idx) {
                // If not ignoring still, confirm filters now
                if let Some(buffer_whitelist) = self.parser_options.filter.buffer_whitelist.as_ref() {
                    if !self.key_path.match_list(buffer_whitelist.iter().collect(), true) {
                        current_node.node_ignore_buffer = true;
                    }
                }
            }
        }
        // Register element begin events
        if let Some(list_maps_for_event) = self.event_map.get(&ParserEvent::OnElementBegin) {
            for event_key in list_maps_for_event.keys() {
                if self.key_path.match_expr(event_key, false) {
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
        let ignoring_current_buffer = self.is_ignoring_current_buffer();
        if let Some(value_buffer) = self.value_buffer.as_mut() {
            value_buffer.pointer_down(key, !ignoring_current_buffer).unwrap(); // Panic here represents a logical error : if identified, to be fixed
            if !ignoring_current_buffer {
                // Only write buffer if not ignoring current
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
    }

    #[inline]
    fn save_value(
        &mut self,
        idx: usize,
        parent_status: &Status,
        mut output_value: Option<Rc<Value>>,
        buffer_value: Option<Rc<Value>>,
        move_up_value: Option<Rc<Value>>,
        new_node_idx: usize // This may be different from idx which represents the object new node is being attached to
    ) -> Result<Option<String>, ParseError> {
        // Cannot use self.is_ignoring_current_output() because current_idx is not always the node we are saving
        let buffer_value = if self.node_map.get(&new_node_idx).map(|node| node.node_ignore_buffer).unwrap_or(false) {
            None
        } else {
            buffer_value.as_ref()
        };
        self.on_event_value_completed(buffer_value.map(|val| Rc::clone(&val)));
        if self.node_map.get(&new_node_idx).map(|node| node.node_ignore_output).unwrap_or(false) {
            output_value = None;
        }
        self.on_event_move_up(move_up_value);
        Ok(self.parser_output.on_status_complete(parent_status, &self.current_status, idx, output_value))
    }

    #[inline]
    fn on_event_move_up(&mut self, value: Option<Rc<Value>>) {
        if let Some(list_maps_for_event) = self.event_map.get(&ParserEvent::OnElementEnd) {
            for event_key in list_maps_for_event.keys() {
                if self.key_path.match_expr(event_key, false) {
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

    #[inline]
    // This adds additional optional processing, such as buffering the value
    fn on_event_value_completed(&mut self, buffer_value: Option<Rc<Value>>) {
        if let Some(value_buffer) = self.value_buffer.as_mut() {
            if let Some(output_value) = buffer_value {
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

    /// Builds a new subnode with all necessary processing
    /// Returns parent_node_idx
    #[inline]
    fn new_subnode(&mut self, node_type: NodeType, new_status: Status) -> usize {
        let new_node_idx = self.ref_index_generator.generate();
        let parent_node_idx = self.current_node_idx;
        self.current_node_idx = new_node_idx;
        self.current_status = new_status; // Become the new type
        let (node_ignore_output, node_ignore_buffer) = self.node_map
            .get(&parent_node_idx)
            .map(|n| (n.node_ignore_output, n.node_ignore_buffer)).unwrap_or((false, false)); // Init to parent's value
        self.node_map.insert(self.current_node_idx, Node::new(
            Some(parent_node_idx),
            node_type,
            node_ignore_output,
            node_ignore_buffer
        ));
        parent_node_idx
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

            // Init at root level
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
                self.node_map.insert(self.current_node_idx, Node::new(None, new_node_type, false, false));
                let row = self.parser_output.on_init(
                    self.current_node_idx,
                    next_status.as_ref()
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
                    return self.save_value(
                        self.current_node_idx,
                        &Status::None(StatusNone::new()),
                        output_value.as_ref().map(|v| Rc::clone(&v)),
                        output_value.as_ref().map(|v| Rc::clone(&v)),
                        output_value,
                        self.current_node_idx,
                    );
                }
                let current_idx = self.current_node_idx;
                let parent_idx = current_node.parent_idx.unwrap();
                // Already update current_node_idx cursor back to the parent value.
                // For clarity, let's not use it afterwards (use parent_idx instead)
                self.current_node_idx = parent_idx;

                // Because parent_idx exists, parent_node must always be defined
                let parent_node = self.node_map.get_mut(&parent_idx).unwrap();
                match &mut parent_node.node_type {
                    node::NodeType::Object(ref mut potential_key) => {
                        if let Some(key) = potential_key.take() {
                            // The key exists => we are returning from the object value
                            let (
                                save_idx,
                                save_value_output,
                                save_value_buffer
                            ): (usize, Option<Rc<Value>>, Option<Rc<Value>>) = match current_status {
                                // The way to write the row, however, depends on the type
                                // Basic types, we have to append to the parent object itself
                                // Except for the value we buffer, in which case it's straightforward
                                Status::Null(_)|
                                Status::Bool(_) |
                                Status::Number(_) => {
                                    let value = output_value.as_ref().unwrap(); // A basic type, when Done, absolutely returns a value
                                    (
                                        parent_idx,
                                        Some(Rc::new(json!({key: value}))),
                                        Some(Rc::clone(&value))
                                    )
                                },
                                // For strings, we have already initialized it, so append to self
                                Status::String(_) => {
                                    (
                                        current_idx,
                                        output_value.as_ref().map(|v| Rc::clone(v)), // Value might not be present if flushed
                                        Some(output_value.as_ref().map(|v|
                                            Rc::clone(v))
                                            .unwrap_or(Rc::new(Value::String(String::new()))
                                        )), // For the buffer, a String value should always be initialized, at least as empty string
                                    )
                                },
                                Status::Object(_) | Status::Array(_) => {
                                    (
                                        0, // irrelevant here
                                        output_value.as_ref().map(|v| Rc::clone(v)),
                                        output_value
                                    )
                                },
                                _ => unreachable!("All relevant types are covered, aren't they?")
                            };
                            let saved_value = self.save_value(
                                save_idx,
                                &Status::Object(StatusObject::new()),
                                save_value_output.as_ref().map(|v| Rc::clone(&v)),
                                save_value_buffer,
                                save_value_output,
                                current_idx
                            );
                            self.current_status = Status::Object(StatusObject {
                                substatus: SubStatusObject::BeforeKV(status_done.comma_matched)
                            });
                            if status_done.done_object {
                                // Not only the value is completed, but the current object must be too : go back up once again
                                self.move_up();
                            }
                            return saved_value;
                        } else {
                            match current_status {
                                Status::String(_) => {
                                    // Key does not exist yet => we are returning from the String value for the key : save it and continue
                                    let value = output_value.unwrap(); // String value for the object key must exist
                                    let new_key = value.as_str().unwrap().to_string();
                                    let output_on_new_key = self.parser_output.on_object_key_complete(&new_key);
                                    *potential_key = Some(new_key);
                                    self.current_status = Status::Object(StatusObject {
                                        substatus: SubStatusObject::BetweenKV(false)
                                    });
                                    return Ok(output_on_new_key);
                                },
                                _ => unreachable!("Parser logic error : non-string status without potential key")
                            };
                        }
                    },
                    node::NodeType::Array(_) => {
                        let (
                            save_idx,
                            output_value,
                            buffer_value
                        ): (usize, Option<Rc<Value>>, Option<Rc<Value>>) = match current_status {
                            // The way to write the row, however, depends on the type
                            // Basic types, we have to append to the parent object itself
                            Status::Null(_)|
                            Status::Bool(_) |
                            Status::Number(_) => {
                                let value = output_value.as_ref().unwrap();
                                (
                                    parent_idx,
                                    Some(Rc::clone(&value)),
                                    Some(Rc::clone(&value))
                                )
                            },
                            Status::String(_) => {
                                (
                                    current_idx,
                                    // For output, it has already been initialized, so we should not add output if empty
                                    output_value.as_ref().and_then(|v| {
                                        if v.as_str().unwrap().len() > 0 {
                                            Some(Rc::clone(&v))
                                        } else {
                                            None
                                        }
                                    }),
                                    // For buffer, however, we never want None, as it will buffer the empty string as null
                                    Some(output_value.unwrap_or(Rc::new(Value::String(String::new()))))
                                )
                            },
                            Status::Object(_) | Status::Array(_) => {
                                (
                                    0, // irrelevant here
                                    output_value.as_ref().map(|v| Rc::clone(v)),
                                    output_value
                                )
                            },
                            _ => unreachable!("All base types are covered, aren't they?")
                        };
                        let saved_value = self.save_value(
                            save_idx,
                            &Status::Array(StatusArray::new()),
                            output_value.as_ref().map(|v| Rc::clone(&v)),
                            buffer_value,
                            output_value,
                            current_idx
                        );
                        self.current_status = Status::Array(StatusArray { comma_matched: status_done.comma_matched });
                        if status_done.done_array {
                            // Not only the value is completed, but the current array must be too
                            // => go back up once again
                            self.move_up();
                        }
                        return saved_value;
                    },
                    node::NodeType::Basic => {
                        unreachable!("Nested data cannot return into non-object or non-array")
                    },
                }
            },

            // Making a subnode of an object or an array
            (
                _current_status @ (Status::Object(_) | Status::Array(_)),
                Some(new_status)
            ) => {
                let parent_node_idx = match new_status {
                    Status::Null(_) |
                    Status::Bool(_) |
                    Status::Number(_) |
                    Status::String(_) => {
                        self.new_subnode(NodeType::Basic, new_status)
                    }
                    Status::Array(_) => {
                        self.new_subnode(NodeType::Array(0), new_status)
                    },
                    Status::Object(_) => {
                        self.new_subnode(NodeType::Object(None), new_status)
                    },
                    _ => unreachable!("Invalid flow")
                };
                let parent_node = self.node_map.get_mut(&parent_node_idx).unwrap();

                // Some datas change depending on whether we are making a subnode of object or array
                // Parametrize below
                let key_and_row: Option<(String, ParentNode)> = match &mut parent_node.node_type {
                    NodeType::Object(ref potential_key) => { // Only when we are parsing the string that is the value of the object (because key is existing)
                        if let Some(key) = potential_key {
                            let key_copy = key.clone();
                            Some((key.clone(), ParentNode::Object(key_copy)))
                        } else {
                            // This String is being used to parse an object's key : do not write now, wait for the value
                            None
                        }
                    }
                    NodeType::Array(arr_idx) => { // Or when its the value of the array
                        let arr_idx_copy = *arr_idx;
                        *arr_idx = *arr_idx + 1; // Advance index after copy
                        Some((arr_idx_copy.to_string(), ParentNode::Array(arr_idx_copy)))
                    },
                    _ => unreachable!("Logic error : String cannot be a child of non-object and non-array")
                };

                // Write data
                if let Some((key, parent_node)) = key_and_row {  
                    self.on_event_move_down(&key);
                    if !self.is_ignoring_current_output() {
                        Ok(self.parser_output.on_new_subnode(
                            parent_node,
                            &self.current_status,
                            parent_node_idx,
                            self.current_node_idx
                        ))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
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
                self.parser_output.on_flush(self.current_node_idx, &data)
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
                    match &mut parent_node.node_type {
                        NodeType::Object(potential_key) => {
                            *potential_key = None; // When arriving back up in an object, make sure to unset the key (string key case is handled elsewhere)
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

    pub fn set_options(&mut self, parser_options: ParserOptions) {
        self.parser_options = parser_options;
    }

    /// Call this method when all data has been sent. There might be lingering state
    pub fn finish(&mut self) {
        match self.current_status.finish() {
            Ok(final_value) => {
                if let Some(final_value) = final_value {
                    self.on_event_value_completed(Some(Rc::new(final_value)));
                }
            },
            Err(err) => {
                log::error!("{}", err.to_string())
            },
        }
    }
}