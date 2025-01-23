/// Type of node, especially needed when going back up and checking where we're at. Parametrization included
#[derive(Debug)]
pub(crate) enum NodeType {
    Object(Option<String>), // The parameter is a potential key. If exists, then when returning from a subobject, we know that we are dealing with a string key
    Array(usize), // Counting indices
    Basic // A basic type for non-containers
}

/// Structure holding information about the node being written, or previously written
#[derive(Debug)]
pub(crate) struct Node {
    pub parent_idx: Option<usize>, // Pointing the parent node if any. Root does not have a parent node
    pub node_type: NodeType,
}

impl Node {
    pub fn new(parent_idx: Option<usize>, node_type: NodeType) -> Self {
        Self {
            parent_idx,
            node_type,
        }
    }
}