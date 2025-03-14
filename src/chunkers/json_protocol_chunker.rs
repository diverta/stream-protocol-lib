use std::{collections::HashMap, time::Duration};
use std::{task::Poll, thread};
use futures::Stream;
use serde_json::{json, Value};
use unicode_segmentation::UnicodeSegmentation;

use crate::json_stream_parser::parser_output::stream_protocol_output::STREAM_VAR_PREFIX;
use crate::ref_index_generator::RefIndexGenerator;

use super::json_value_pointer::JsonValuePointer;

const MAX_CHUNK_SETTINGS_INTERVAL: usize = 10_000; // Max duration (in ms) that we allow user to wait between each request

/// This chunker chunks a Value (JSON) following the definition of the custom KurocoEdge JsonStream protocol
pub struct JsonProtocolChunker {
    source: Value,
    idx_generator: RefIndexGenerator, // A shared, dynamic counter for new rows
    root_ref_index: usize, // An initially assigned number for this chunker to start with
}

pub struct JsonProtocolChunkIter {
    source: Value,
    idx_generator: RefIndexGenerator,
    root_ref_index: usize,
    current_ref_index: Option<usize>,
    // Map of the next accessed element of a given json pointer expression. For both Arrays & Objects (thanks to IndexMap)
    next_accessed_idx: HashMap<String, usize>,
    pointer: JsonValuePointer, // Points to the current node being iterated on
    pointer_index_map: HashMap<String, usize>, // Map of the ref_index assigned to the pointed level
    buf_size: usize,
}

#[allow(unused)]
pub struct JsonProtocolChunkStream {
    chunker: JsonProtocolChunkIter,
    sleep_duration: Duration,
}

pub enum JsonProtocolChunkOperator {
    Assign,
    Append
}

impl JsonProtocolChunkOperator {
    pub fn get_op(&self) -> &'static str {
        match self {
            JsonProtocolChunkOperator::Assign => "=",
            JsonProtocolChunkOperator::Append => "+=",
        }
    }
}

impl JsonProtocolChunker {
    pub fn new(source: Value, idx_generator: RefIndexGenerator, root_ref_index: usize) -> Self {
        JsonProtocolChunker {
            source,
            idx_generator,
            root_ref_index
        }
    }

    pub fn chunks(self, buf_size: usize) -> JsonProtocolChunkIter {
        JsonProtocolChunkIter::new(self.source, buf_size, self.idx_generator.clone(), self.root_ref_index)
    }

    pub fn stream(self, buf_size: usize, sleep_interval: usize) -> JsonProtocolChunkStream {
        let chunker = self.chunks(buf_size);
        let sleep_interval = std::cmp::min(sleep_interval, MAX_CHUNK_SETTINGS_INTERVAL);
        JsonProtocolChunkStream {
            chunker,
            sleep_duration: Duration::from_millis(sleep_interval.try_into().unwrap()),
        }
    }
}

impl JsonProtocolChunkIter {
    pub fn new(source: Value, buf_size: usize, idx_generator: RefIndexGenerator, root_ref_index: usize) -> Self {
        JsonProtocolChunkIter {
            // Init with empty value of the same type
            source,
            pointer: JsonValuePointer {
                pointer_expr: None,
            },
            next_accessed_idx: HashMap::new(),
            pointer_index_map: HashMap::new(),
            current_ref_index: Some(0),
            buf_size,
            idx_generator,
            root_ref_index
        }
    }
}

/// The iterator's Item is a partial row to be printed as the next yield following the protocol's definition
impl Iterator for JsonProtocolChunkIter {
    type Item = String;
    
    fn next(&mut self) -> Option<Self::Item> {
        // Compare source and current values at location pointed at
        let source_val: &Value = match &self.pointer.pointer_expr {
            Some(p) => {
                self.source.pointer(&p).unwrap() // If this panics, there is an issue with the logic
            }
            None => {
                &self.source
            }
        };

        let pointer_expr = if let Some(pointer_expr) = self.pointer.pointer_expr.as_ref() {
            pointer_expr.clone()
        } else {
            "/".to_string() // Root
        };
        
        if self.current_ref_index.is_none() {
            self.current_ref_index = Some(self.idx_generator.generate());
        }

        let current_idx = self
            .pointer_index_map
            .entry(pointer_expr.clone())
            .or_insert(
                // Default index : if we are at root, then use root_ref_index
                if pointer_expr == "/" { self.root_ref_index }
                else { self.current_ref_index.unwrap() }
            );

        match source_val {
            Value::Null | Value::Bool(_) | Value::Number(_) => {
                let next_accessed = self.next_accessed_idx.get(pointer_expr.as_str()).map(|v| *v);
                if next_accessed.is_some() {
                    // Null/Bool/Number processing ended : either go up one level, or terminate if at root
                    // Also cleanup next_accessed_idx
                    self.next_accessed_idx.remove(pointer_expr.as_str());
                    if pointer_expr.as_str() == "/" {
                        return None;
                    } else {
                        self.pointer.up();
                        return self.next();
                    }
                } else {
                    self.next_accessed_idx.insert(pointer_expr, 0); // Doesn't matter the value, just mark this node as processed
                    Some(format!("{}{}{}\n",
                        current_idx,
                        JsonProtocolChunkOperator::Assign.get_op(),
                        source_val.to_string()
                    ))
                }
            },
            Value::String(s) => {
                let next_accessed = self.next_accessed_idx.get(pointer_expr.as_str()).map(|v| *v);
                if let Some(next_accessed) = next_accessed {
                    if next_accessed >= s.len() {
                        // String processing ended : either go up one level, or terminate if at root
                        // Also cleanup next_accessed_idx
                        self.next_accessed_idx.remove(pointer_expr.as_str());
                        if pointer_expr.as_str() == "/" {
                            return None;
                        } else {
                            self.pointer.up();
                            return self.next();
                        }
                    }
                    let s = &s[next_accessed..];
                    let mut buf: String;
                    // First time looking at this string
                    let output = if self.buf_size >= s.as_bytes().len() {
                        self.next_accessed_idx.entry(pointer_expr).and_modify(|v| *v = *v + s.as_bytes().len());
                        s
                    } else {
                        let mut idx: usize = 0;
                        buf = String::with_capacity(self.buf_size);
                        for grapheme in s.graphemes(true) {
                            buf.push_str(grapheme);
                            idx += grapheme.len();
                            if idx >= self.buf_size {
                                self.next_accessed_idx.entry(pointer_expr).and_modify(|v| *v = *v + idx);
                                break;
                            }
                        }
                        &buf
                    };
                    Some(format!("{}{}{}\n",
                        current_idx,
                        JsonProtocolChunkOperator::Append.get_op(),
                        Value::String(output.to_string()).to_string()
                    ))
                } else {
                    let mut buf: String;
                    // First time looking at this string
                    let output = if self.buf_size >= s.as_bytes().len() {
                        self.next_accessed_idx.insert(pointer_expr, s.as_bytes().len());
                        self.pointer.up();
                        s
                    } else {
                        let mut idx: usize = 0;
                        buf = String::with_capacity(self.buf_size);
                        for grapheme in s.graphemes(true) {
                            buf.push_str(grapheme);
                            idx += grapheme.len();
                            if idx >= self.buf_size {
                                self.next_accessed_idx.insert(pointer_expr, idx);
                                break;
                            }
                        }
                        &buf
                    };
                    Some(format!("{}{}{}\n",
                        current_idx,
                        JsonProtocolChunkOperator::Assign.get_op(),
                        Value::String(output.to_string()).to_string()
                    ))
                }
            },
            Value::Array(arr) => {
                let next_accessed = self.next_accessed_idx.get(pointer_expr.as_str()).map(|v| *v);
                if let Some(next_accessed) = next_accessed {
                    if next_accessed >= arr.len() {
                        // Array processing ended : either go up one level, or terminate if at root
                        // Also cleanup next_accessed_idx
                        self.next_accessed_idx.remove(pointer_expr.as_str());
                        if pointer_expr.as_str() == "/" {
                            return None;
                        } else {
                            self.pointer.up();
                            return self.next();
                        }
                    }
                    // We should proceed here immediately after the else below (on the next iteration), if the map is not empty
                    self.next_accessed_idx.insert(pointer_expr, next_accessed + 1); // Make sure the next one is now updated (for the future iter)
                    // Scroll through the array to get to the next_accessed element
                    if arr.len() > next_accessed {
                        self.pointer.down(next_accessed.to_string().as_str());
                        self.current_ref_index = Some(self.idx_generator.generate());
                        return Some(format!("{}{}{}\n",
                            current_idx,
                            JsonProtocolChunkOperator::Append.get_op(),
                            Value::String(format!("{}{}", STREAM_VAR_PREFIX, self.current_ref_index.unwrap()))
                        ));
                    }
                    // Array processing ended
                    self.pointer.up();
                    return self.next();
                } else {
                    // This is the first time we are looking at this array : parametrize empty
                    self.next_accessed_idx.insert(pointer_expr, 0);
                    if arr.len() == 0 {
                        // Empty array : move pointer back
                        self.pointer.up();
                    }
                    Some(format!("{}{}{}\n",
                        current_idx,
                        JsonProtocolChunkOperator::Assign.get_op(),
                        "[]"
                    ))
                }
            },
            Value::Object(map) => {
                let next_accessed = self.next_accessed_idx.get(pointer_expr.as_str()).map(|v| *v);
                if let Some(next_accessed) = next_accessed {
                    if next_accessed >= map.len() {
                        // Map processing ended : either go up one level, or terminate if at root
                        // Also cleanup next_accessed_idx
                        self.next_accessed_idx.remove(pointer_expr.as_str());
                        if pointer_expr.as_str() == "/" {
                            return None;
                        } else {
                            self.pointer.up();
                            return self.next();
                        }
                    }
                    // We should proceed here immediately after the else below (on the next iteration), if the map is not empty
                    self.next_accessed_idx.insert(pointer_expr, next_accessed + 1); // Make sure the next one is now updated (for the future iter)
                    // Scroll through the map to get to the next_accessed element
                    for (current_map_idx, (key, _)) in map.iter().enumerate() {
                        if next_accessed == current_map_idx {
                            // This element can now be processed
                            self.pointer.down(&key);
                            self.current_ref_index = Some(self.idx_generator.generate());
                            let value_parametrized = format!("{}{}", STREAM_VAR_PREFIX, self.current_ref_index.unwrap());
                            return Some(format!("{}{}{}\n",
                                current_idx,
                                JsonProtocolChunkOperator::Append.get_op(),
                                json!({key: value_parametrized}).to_string()
                            ));
                        } else {
                            continue;
                        }
                    }
                    // Map processing ended
                    self.pointer.up();
                    return self.next();
                } else {
                    // This is the first time we are looking at this object : parametrize as empty
                    self.next_accessed_idx.insert(pointer_expr, 0);
                    if map.len() == 0 {
                        // Empty map : move pointer back
                        self.pointer.up();
                    }
                    Some(format!("{}{}{}\n",
                        current_idx,
                        JsonProtocolChunkOperator::Assign.get_op(),
                        "{}"
                    ))
                }
            },
        }
    }
}

impl Stream for JsonProtocolChunkStream {
    type Item = String;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        // Unfortunately in a single threaded context,
        // there is no other wait to await asynchonously other than putting the whole thread to sleep
        thread::sleep(self.sleep_duration);
        Poll::Ready(self.chunker.next())
    }
}