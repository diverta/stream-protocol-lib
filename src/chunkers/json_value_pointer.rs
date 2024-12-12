// Struct allowing to navigate the Value tree
// Lookup serde_json::Value::pointer() function for the syntaxis details
pub struct JsonValuePointer {
    pub pointer_expr: Option<String> // It must never end with "/". None means root
}

impl JsonValuePointer {
    /// Returns parent expression if any, or None if root
    /// (!) Assumes a non-root expression never ends with /
    pub fn parent_expr(&self) -> Option<String> {
        self.pointer_expr.as_ref().and_then(|expr| {
            let expr_parts = expr.split("/").collect::<Vec<&str>>();
            expr_parts.split_last().and_then(|(_, all_without_last)| {
                let joined = all_without_last.join("/");
                match joined.as_str() {
                    "" => None,
                    _ => Some(joined)
                }
            })
        })
    }

    /// Moves one layer down, by appending a new key
    pub fn down(&mut self, key: &str) {
        match self.pointer_expr.as_mut() {
            Some(pointer) => {
                *pointer = format!("{}/{}", pointer, key);
            },
            None => {
                self.pointer_expr = Some(format!("/{}", key))
            },
        }
    }

    /// Moves one layer up. If not possible (already at root), returns None
    pub fn up(&mut self) -> Option<()> {
        if let Some(parent_expr) = self.parent_expr() {
            self.pointer_expr = Some(parent_expr);
            Some(())
        } else {
            self.pointer_expr = None;
            None
        }
    }
}