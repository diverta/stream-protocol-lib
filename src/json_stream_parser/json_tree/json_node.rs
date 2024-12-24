use std::{cell::RefCell, fmt::{self, Display}, io, rc::{Rc, Weak}};

use serde::Serialize;
use serde_json::{to_writer, to_writer_pretty};

use crate::json_stream_parser::error::LogicalError;

use super::json_value::JsonValue;

pub(crate) struct JsonNode {
    pub(crate) value: RefCell<JsonValue>,
    pub(crate) parent: RefCell<Weak<JsonNode>>,
}

impl JsonNode {
    pub(crate) fn new(value: JsonValue) -> Self {
        JsonNode {
            value: RefCell::new(value),
            parent: RefCell::new(Weak::new())
        }
    }   

    pub(crate) fn set_value(&self, value: JsonValue) {
        let mut value_ref = self.value.borrow_mut();
        *value_ref = value;
    }

    pub(crate) fn set_parent(&self, parent: Rc<JsonNode>) {
        let mut parent_ref = self.parent.borrow_mut();
        *parent_ref = Rc::downgrade(&parent)
    }

    pub(crate) fn append_to_array(self: Rc<Self>, value: Rc<JsonNode>) -> Result<(), LogicalError> {
        let mut value_ref = self.value.borrow_mut();
        match &mut *value_ref {
            JsonValue::Array(arr) => {
                value.set_parent(Rc::clone(&self));
                arr.push(value);
                Ok(())
            },
            _ => Err(LogicalError::new("Attempting to append a new value to a non array".to_owned()))
        }
    }

    pub(crate) fn add_to_object(self: Rc<Self>, key: String, value: Rc<JsonNode>) -> Result<(), LogicalError> {
        let mut value_ref = self.value.borrow_mut();
        match &mut *value_ref {
            JsonValue::Object(obj) => {
                value.set_parent(Rc::clone(&self));
                obj.insert(key, value);
                Ok(())
            },
            _ => Err(LogicalError::new("Attempting to add a new key to a non object".to_owned()))
        }
    }
}

impl Serialize for JsonNode {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl Display for JsonNode {
    // Display JsonValue as a string
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct WriterFormatter<'a, 'b: 'a> {
            inner: &'a mut fmt::Formatter<'b>,
        }

        impl<'a, 'b> io::Write for WriterFormatter<'a, 'b> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                // Safety: the serializer below only emits valid utf8 when using
                // the default formatter.
                let s = unsafe { std::str::from_utf8_unchecked(buf) };
                tri!(self.inner.write_str(s).map_err(io_error));
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        fn io_error(_: fmt::Error) -> io::Error {
            // Error value does not matter because Display impl just maps it
            // back to fmt::Error.
            io::Error::new(io::ErrorKind::Other, "fmt error")
        }

        let alternate = f.alternate();
        let mut wr = WriterFormatter { inner: f };
        if alternate {
            // {:#}
            to_writer_pretty(&mut wr, self).map_err(|_| fmt::Error)
        } else {
            // {}
            to_writer(&mut wr, self).map_err(|_| fmt::Error)
        }
    }
}