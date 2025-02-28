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
    pub node_ignore_output: bool, // Registering within the node whether this node is ignoring outputting data being parsed
    pub node_ignore_buffer: bool, // Registering within the node whether this node is ignoring buffering data being parsed
}

impl Node {
    pub fn new(
        parent_idx: Option<usize>,
        node_type: NodeType,
        node_ignore_output: bool,
        node_ignore_buffer: bool
    ) -> Self {
        Self {
            parent_idx,
            node_type,
            node_ignore_output,
            node_ignore_buffer
        }
    }
}