use std::{io::Error, task::Poll};

use futures::AsyncWrite;
use partial_json_protocol_mapper::PartialJsonProtocolMapper;
use pin_project::pin_project;
use status::{Status, StatusTrait};

use crate::{byte_to_char, ref_index_generator::RefIndexGenerator};

pub(crate) mod error;
//pub(crate) mod json_tree;
pub(crate) mod partial_json_protocol_mapper;
pub(crate) mod status;

#[pin_project]
pub struct JsonStreamParser<W> {
    mapper: PartialJsonProtocolMapper,
    current_status: Status,
    char_pos: usize,
    #[pin]
    writer: W,
}

impl<W> JsonStreamParser<W>
where W: AsyncWrite {
    pub fn new(ref_index_generator: RefIndexGenerator, writer: W, current_node_index: usize) -> JsonStreamParser<W> {
        JsonStreamParser {
            mapper: PartialJsonProtocolMapper::new(ref_index_generator, current_node_index),
            current_status: Status::new(),
            char_pos: 0,
            writer
        }
    }

    /// Call it when a bytes buffer has been exhausted, and a pause is expected before receiving the next buffer
    pub fn flush(&self) {

    }
}

impl<W> AsyncWrite for JsonStreamParser<W>
where W: AsyncWrite {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        let mut final_output: String = String::new();
        for c in buf {
            match this.mapper.add_char(c) {
                Ok(Some(output)) => {
                    final_output.push_str(&output.into());
                }
                Ok(None) => {
                    // Not ready to write the line yet, do nothing
                }
                Err(err) => {
                    log::error!("JSON parse error at character '{}' : {}", byte_to_char(c), err.msg);
                    return Poll::Ready(Err(Error::other(format!("JSON Parse error : {}", err.msg))));
                },
            }
        }
        
        this.writer.poll_write(cx, final_output.as_bytes())
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        if let Some(out) = this.mapper.flush() {
            this.writer.poll_write(cx, &out).map(|result| result.map(|_| (())))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        this.writer.poll_close(cx)
    }
}