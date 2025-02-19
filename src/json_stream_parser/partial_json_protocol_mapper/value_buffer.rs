use serde_json::Value;

/// Helper struct which builds a Value while going through partial JSON
/// It is used optionally if enable_buffering flag is set
pub(crate) struct ValueBuffer {
    pub(crate) root: Value,
    // serde_json::Value pointer as per https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html#method.pointer
    pub(crate) pointer: String,
}

impl ValueBuffer {
    pub fn new(root: Value) -> Self {
        Self {
            root,
            pointer: Self::pointer_init(),
        }
    }

    pub fn pointer_init() -> String {
        String::new() // Uniformize in case adjustments are needed
    }

    /// Moves out data, and resetting the ValueBuffer
    pub fn take_buffered_data(&mut self) -> Value {
        let mut root_value = Value::Null;
        std::mem::swap(&mut root_value, &mut self.root);
        self.pointer = Self::pointer_init();
        return root_value;
    }

    pub fn pointer_up(&mut self) {
        if let Some(last_slash_pos) = self.pointer.rfind('/') {
            self.pointer.replace_range(last_slash_pos.., "");
        }
    }

    // Not only moves pointer down, but optionally also inserts null value there - as it is expected for it to be replaced
    // Not inserting the placeholder is useful when we are filtering the values, keeping the pointer as-is but without extra insertion
    pub fn pointer_down(&mut self, key: &str, insert_placeholder: bool) -> Result<(), String> {
        // Validation that going down is applicable
        if insert_placeholder {
            match self.root.pointer_mut(&self.pointer) {
                Some(current_value) => {
                    match current_value {
                        Value::Null |
                        Value::Bool(_) |
                        Value::Number(_) |
                        Value::String(_) => {
                            return Err(format!("ValueBuffer: Trying to go down non object or non array"))
                        }
                        Value::Array(arr) => {
                            match key.parse::<usize>() {
                                Ok(key_int) => {
                                    if key_int == arr.len() {
                                        (*arr).push(Value::Null);
                                    } else {
                                        return Err(format!("ValueBuffer: Trying to add invalid array item"))
                                    }
                                },
                                Err(_) => {
                                    return Err(format!("ValueBuffer: Trying to go down non integer array key"))
                                }
                            }
                        },
                        Value::Object(map) => {
                            map.insert(key.to_owned(), Value::Null);
                        },
                    }
                },
                None => {
                    return Err(format!("ValueBuffer: No element at the current pointer"))
                },
            }
        }
        // Update pointer
        self.pointer = format!("{}/{key}", self.pointer);
        Ok(())
    }

    pub fn insert_at_pointer(&mut self, value: Value) -> Result<(), String> {
        match self.root.pointer_mut(&self.pointer) {
            Some(existing_value) => {
                *existing_value = value;
                Ok(())
            },
            None => {
                Err(format!("ValueBuffer: unable to insert value at pointer {}", &self.pointer))
            },
        }
    }
}