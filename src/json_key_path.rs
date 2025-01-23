/// String representation of a path to a JSON key, allowing navigation
/// Examples :
/// parent    # Root is an object with a key
/// parent.child.name
/// parent.children.1.name
/// 1       # Root being an array, access its second element
/// parent.1 # Root is an object having key "parent" which is an array or an object
/// parent.*  # Wildcard for any key of 'parent' which is object or array
/// *         # Wildcard for any key of root which is an object or array
/// *.*      # WILL NOT WORK ! Multiple wildcards are not supported
pub struct JsonKeyPath {
    current_key: String
}

impl JsonKeyPath {
    pub fn new() -> Self {
        Self {
            current_key: String::new()
        }
    }

    pub fn get_current_key(&self) -> &str {
        self.current_key.as_str()
    }

    /// Moving down an object with key
    pub fn move_down_object_or_array(&mut self, key: &str) -> bool {
        if key.len() == 0 {
            return false;
        }
        if self.current_key.len() > 0 {
            self.current_key.push('.');
        }
        self.current_key.push_str(key);
        true
    }

    /// Moving up one level
    /// Returns false if trying to move up at root, or any malformed key expression
    pub fn move_up(&mut self) -> bool {
        if self.current_key.len() == 0 {
            return false;
        }
        let mut last_dot_position = 0;
        
        for (indice, char_value) in self.current_key.char_indices().rev() {
            // If 0 is reached, we moved up to root
            if char_value == '.' {
                last_dot_position = indice;
                break;
            }
        }
        
        self.current_key.drain(last_dot_position..);
        true
    }

    /// Matches an expression which may include a wildcard. Refer to doc of JsonKeyPath for more details
    pub fn match_expr(&self, expr: &str) -> bool {
        // Two cursors that increase at the same time
        let mut expr_idx = 0_usize; 
        let mut current_idx = 0_usize;
        let mut wildcard_found = false; // Ensure we only have a single wildcard
        let expr_bytes = expr.as_bytes();
        let expr_bytes_len = expr_bytes.len();
        let current_bytes = self.current_key.as_bytes();
        let current_bytes_len = current_bytes.len();
        loop {
            if expr_bytes_len == expr_idx || current_bytes_len == current_idx {
                // Reached max length for either : only return true if the max length is also reached for the other one
                return expr_bytes_len == expr_idx && current_bytes_len == current_idx;
            }
            if expr_bytes[expr_idx] == b'*' {
                // Handle wildcard
                if wildcard_found {
                    // Aleady found : this is a second one
                    return false;
                }
                wildcard_found = true;
                loop {
                    // Forward current_idx to match the state
                    current_idx += 1;
                    if current_bytes_len == current_idx || current_bytes[current_idx] == b'.' {
                        // Either reached end of current string, or a dot : advance the other cursor once, and reloop
                        expr_idx += 1;
                        break;
                    }
                }
            } else {
                // No wildcard
                if expr_bytes[expr_idx] != current_bytes[current_idx] {
                    return false;
                }
                expr_idx += 1;
                current_idx += 1;
            }
        }
    }
}