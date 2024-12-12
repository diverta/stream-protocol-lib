use serde::Deserialize;

pub mod json_value_pointer;
pub mod json_growing_tree_chunker;
pub mod json_protocol_chunker;

#[derive(Deserialize, Debug, PartialEq)]
pub struct DataSourceChunkSettings {
    pub interval: usize, // In milliseconds
    pub buf_size: usize,
}