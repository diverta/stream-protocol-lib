use std::ascii::EscapeDefault;

pub mod chunkers;
pub mod ref_index_generator;
pub mod json_stream_parser;
pub mod json_key_path;

pub fn byte_to_char(byte: &u8) -> EscapeDefault {
    std::ascii::escape_default(*byte)
}