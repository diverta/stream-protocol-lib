use derivative::Derivative;
use error::ParseError;
use partial_json_protocol_mapper::PartialJsonProtocolMapper;
use serde_json::Value;
use status::{Status, StatusTrait};

use crate::{byte_to_char, ref_index_generator::RefIndexGenerator};

pub(crate) mod error;
//pub(crate) mod json_tree;
pub(crate) mod partial_json_protocol_mapper;
pub(crate) mod status;

use std::rc::Rc;
#[cfg( feature = "async" )] use std::{io::Error, task::Poll};
#[cfg( feature = "async" )] use futures::AsyncWrite;
#[cfg( feature = "async" )] use pin_project::pin_project;
#[cfg( feature = "async" )]
#[pin_project]
pub struct JsonStreamParser<W, F> {
    mapper: PartialJsonProtocolMapper<F>,
    current_status: Status,
    char_pos: usize,
    #[pin]
    writer: W,
}

#[cfg(not(feature = "async" ))]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct JsonStreamParser<F> {
    #[derivative(Debug="ignore")]
    mapper: PartialJsonProtocolMapper<F>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ParserEvent {
    OnElementBegin,
    OnElementEnd
}

#[cfg( not(feature = "async") )]
impl<F> JsonStreamParser<F>
where F: Fn(Option<Rc<Value>>) -> ()
{
    pub fn new(
        ref_index_generator: RefIndexGenerator,
        current_node_index: usize,
        enable_buffering: bool,
        filter_elements: Option<Vec<String>>,
    ) -> JsonStreamParser<F> {
        JsonStreamParser {
            mapper: PartialJsonProtocolMapper::new(
                ref_index_generator,
                current_node_index,
                enable_buffering,   
                filter_elements
            )
        }
    }

    #[inline]
    pub fn add_char(&mut self, c: &u8) -> Result<Option<String>, ParseError> {
        match self.mapper.add_char(c) {
            Ok(Some(output)) => {
                Ok(Some(output.into()))
            }
            Ok(None) => {
                // Not ready to write the line yet, do nothing
                Ok(None)
            }
            Err(err) => {
                log::error!("JSON parse error at character '{}' : {}", byte_to_char(c), err.msg);
                Err(err)
            },
        }
    }

    #[inline]
    pub fn flush(&mut self) -> Option<String> {
        self.mapper.flush()
    }

    /// Attach a function to be executed when an event occurs at a given element
    /// Element is a simple string path to a JSON key. Ex: "parent.child.grandchildren.0.name"
    /// Json key path uses dot notation to separate levels. Array index can be replaced with wildcard (*) to match every element
    /// The parameter to the event function is set to the value of the element when the event is OnElementEnd,
    /// but only when the element is not an Array or an Object - for performance reasons (set to None in all other cases)
    pub fn add_event_handler(&mut self, event: ParserEvent, element: String, func: F) {
        self.mapper.add_event_handler(event, element, func);
    }

    /// Uses a list of elements to filter the partial JSON to keep
    /// If this function is used, then by default the parser will not output a JSON unless it matches any element of the filter
    /// Element is a simple string path to a JSON key. Ex: "parent.child.grandchildren.0.name"
    pub fn set_filter(&mut self, filter_elements: Vec<String>) {
        self.mapper.set_filter(filter_elements);
    }

    /// Builder style version of add_event_handler method
    pub fn with_event_handler(mut self, event: ParserEvent, element: String, func: F) -> Self {
        self.mapper.add_event_handler(event, element, func);
        self
    }
    
    pub fn get_buffered_data(&self) -> Option<&Value> {
        self.mapper.get_buffered_data()
    }
    
    pub fn take_buffered_data(&mut self) -> Option<Value> {
        self.mapper.take_buffered_data()
    }
}

#[cfg(feature = "async")]
impl<W, F> JsonStreamParser<W, F>
where
    W: AsyncWrite,
    F: Fn(Option<Rc<Value>>) -> ()
{
    pub fn new(ref_index_generator: RefIndexGenerator, writer: W, current_node_index: usize) -> JsonStreamParser<W, F> {
        JsonStreamParser {
            mapper: PartialJsonProtocolMapper::new(ref_index_generator, current_node_index),
            current_status: Status::new(),
            char_pos: 0,
            writer
        }
    }

    #[inline]
    pub fn add_char(&mut self, c: &u8) -> Result<Option<String>, ParseError> {
        match self.mapper.add_char(c) {
            Ok(Some(output)) => {
                Ok(Some(output.into()))
            }
            Ok(None) => {
                // Not ready to write the line yet, do nothing
                Ok(None)
            }
            Err(err) => {
                log::error!("JSON parse error at character '{}' : {}", byte_to_char(c), err.msg);
                Err(err)
            },
        }
    }

    #[inline]
    pub fn flush(&mut self) -> Option<String> {
        self.mapper.flush()
    }

    /// Attach a function to be executed when an event occurs at a given element
    /// Element is a simple string path to a JSON key. Ex: "parent.child.grandchildren.0.name"
    /// Json key path uses dot notation to separate levels. Array index can be replaced with wildcard (*) to match every element
    /// The parameter to the event function is set to the value of the element when the event is OnElementEnd,
    /// but only when the element is not an Array or an Object - for performance reasons (set to None in all other cases)
    pub fn add_event_handler(&mut self, event: ParserEvent, element: String, func: F) {
        self.mapper.add_event_handler(event, element, func);
    }

    /// Builder style version of add_event_handler method
    pub fn with_event_handler(mut self, event: ParserEvent, element: String, func: F) -> Self {
        self.mapper.add_event_handler(event, element, func);
        self
    }
}

#[cfg( feature = "async" )]
impl<W, F> AsyncWrite for JsonStreamParser<W, F>
where
    W: AsyncWrite,
    F: Fn(Option<Rc<Value>>) -> ()
{
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
            this.writer.poll_write(cx, out.as_bytes()).map(|result| result.map(|_| (())))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        this.writer.poll_close(cx)
    }
}