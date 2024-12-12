use std::{cell::RefCell, rc::Rc};

use serde_json::{Map, Value};
use unicode_segmentation::UnicodeSegmentation;

use crate::chunkers::json_value_pointer::JsonValuePointer;

pub struct JsonGrowingTreeChunker<'a> {
    source: &'a Value,
}

pub struct JsonGrowingTreeChunkIter<'a> {
    source: &'a Value,
    current: Rc<RefCell<Value>>, // Holding the value representing the current chunk, starting from empty and growing up to source during iteration
    pointer: JsonValuePointer,
    buf_size: usize
}

impl<'a> JsonGrowingTreeChunker<'a> {
    pub fn new(source: &'a Value) -> Self {
        JsonGrowingTreeChunker {
            source
        }
    }

    pub fn chunks(&self, buf_size: usize) -> JsonGrowingTreeChunkIter {
        let init_current = Self::init_copy(&self.source);
        JsonGrowingTreeChunkIter {
            // Init with empty value of the same type
            source: &self.source,
            current: Rc::new(RefCell::new(init_current)),
            pointer: JsonValuePointer {
                pointer_expr: None,
            },
            buf_size
        }
    }

    fn init_copy(v: &Value) -> Value {
        match v {
            Value::String(_) => Value::String(String::new()),
            Value::Array(_) => Value::Array(Vec::new()),
            Value::Object(_) => Value::Object(Map::new()),
            other => other.clone()
        }
    }
}

impl<'a> Iterator for JsonGrowingTreeChunkIter<'a> {
    type Item = Rc<RefCell<Value>>;

    // Gradually fills current Value, but always returns reference to it
    fn next(&mut self) -> Option<Self::Item> {
        // Compare source and current values at location pointed at
        let source_val: &Value = match &self.pointer.pointer_expr {
            Some(p) => {
                self.source.pointer(&p).unwrap() // If this panics, there is an issue with the logic
            }
            None => {
                self.source
            }
        };
        let potential_parent; // If currently at source then None, otherwise the source.
        let mut current_borrowed = self.current.borrow_mut();
        let current_val: &mut Value = match &self.pointer.pointer_expr {
            Some(p) => {
                potential_parent = Some(Rc::clone(&self.current));
                current_borrowed.pointer_mut(&p).unwrap() // If this panics, there is an issue with the logic
            }
            None => {
                potential_parent = None;
                &mut current_borrowed
            },
        };
        match source_val {
            Value::Null | Value::Bool(_) | Value::Number(_) => {
                // These are not chunkable, and have been initialized already. So simply go up for this iteration
                drop(current_borrowed); // Explicitely removing the borrow asap
                match self.pointer.up() {
                    Some(_) => {
                        // Current layer is done, continue iterating the parent
                        self.next(); // Skip moving up step
                        Some(Rc::clone(&self.current))
                    },
                    None => potential_parent,
                }
            },
            Value::String(source_str) => {
                let current_str = current_val.as_str().unwrap();
                if source_str.len() > current_str.len() {
                    // Append up to remaining buf_size characters
                    let mut current_str = current_str.to_string();
                    let remaining = &source_str[current_str.len()..];
                    if remaining.len() > self.buf_size {
                        // Now, because we are dealing with UTF8, we should not use bytes or chars for measurement, but graphemes
                        let mut total_added = 0usize;
                        for grapheme in remaining.graphemes(true) {
                            current_str.push_str(grapheme); // At least one grapheme will always be pushed
                            total_added += grapheme.len();
                            if total_added >= self.buf_size {
                                break;
                            }
                        }
                        *current_val = Value::String(current_str);
                        drop(current_borrowed); // Explicitely removing the borrow asap

                        return Some(Rc::clone(&self.current));
                    } else {
                        // Enough to finish working 
                        current_str.extend(remaining.chars());
                        *current_val = Value::String(current_str);
                        drop(current_borrowed); // Explicitely removing the borrow asap

                        // Chunking done, go up
                        match self.pointer.up() {
                            Some(_) => {
                                self.next(); // Skip moving up step
                                Some(Rc::clone(&self.current)) // Current layer is done, continue iterating the parent
                            }
                            None => potential_parent,
                        }
                    }
                } else {
                    // Chunking done, go up - although this case shouldn't be reached
                    drop(current_borrowed); // Explicitely removing the borrow asap
                    match self.pointer.up() {
                        Some(_) => {
                            self.next(); // Skip moving up step
                            Some(Rc::clone(&self.current)) // Current layer is done, continue iterating the parent
                        },
                        None => potential_parent,
                    }
                }
            },
            Value::Array(source_array) => {
                let current_arr = current_val.as_array_mut().unwrap(); // If this panics, there is an issue with the logic
                if source_array.len() > current_arr.len() {
                    // There are still items to iterate over inside current array
                    let next_idx = current_arr.len();
                    current_arr.push(JsonGrowingTreeChunker::init_copy(source_array.get(next_idx).unwrap()));
                    drop(current_borrowed); // Explicitely removing the borrow asap

                    self.pointer.down(next_idx.to_string().as_str());
                    return Some(Rc::clone(&self.current));
                } else {
                    // At this point, source & current objects match at the given pointer. Go up
                    drop(current_borrowed); // Explicitely removing the borrow asap
                    match self.pointer.up() {
                        Some(_) => {
                            self.next(); // Skip moving up step
                            Some(Rc::clone(&self.current))  // Current layer is done, continue iterating the parent
                        },
                        None => potential_parent,
                    }
                }
            },
            Value::Object(source_obj) => {
                let current_obj = current_val.as_object_mut().unwrap(); // If this panics, there is an issue with the logic
                for source_key in source_obj.keys() {
                    // If a key exists in current, then its contents are already iterated
                    if current_obj.get(source_key).is_none() {
                        // Key does not exist => nesting
                        current_obj.insert(
                            source_key.clone(),
                            JsonGrowingTreeChunker::init_copy(source_obj.get(source_key).unwrap())
                        );
                        drop(current_borrowed); // Explicitely removing the borrow asap
                        self.pointer.down(source_key);
                        return Some(Rc::clone(&self.current));
                    }
                }
                // At this point, source & current objects match at the given pointer. Go up
                drop(current_borrowed); // Explicitely removing the borrow asap
                match self.pointer.up() {
                    Some(_) => {
                        self.next(); // Skip moving up step
                        Some(Rc::clone(&self.current)) // Current layer is done, continue iterating the parent
                    }
                    None => potential_parent,
                }
            },
        }
    }
}