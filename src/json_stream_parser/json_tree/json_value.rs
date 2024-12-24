use core::fmt;
use std::{fmt::Display, io, rc::Rc};

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{to_writer, to_writer_pretty, Number};

use super::json_node::JsonNode;

pub(crate) enum JsonValue {
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
    Array(Vec<Rc<JsonNode>>),
    Object(IndexMap<String, Rc<JsonNode>>)
}

impl JsonValue {
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }
    pub fn as_null(&self) -> Option<()> {
        match self {
            JsonValue::Null => Some(()),
            _ => None,
        }
    }
    pub fn is_boolean(&self) -> bool {
        self.as_boolean().is_some()
    }
    pub fn as_boolean(&self) -> Option<&bool> {
        match self {
            JsonValue::Boolean(val) => Some(val),
            _ => None,
        }
    }
    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }
    pub fn as_number(&self) -> Option<&Number> {
        match self {
            JsonValue::Number(val) => Some(val),
            _ => None,
        }
    }
    pub fn is_string(&self) -> bool {
        self.as_string().is_some()
    }
    pub fn as_string(&self) -> Option<&String> {
        match self {
            JsonValue::String(val) => Some(val),
            _ => None,
        }
    }
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }
    pub fn as_array(&self) -> Option<&Vec<Rc<JsonNode>>> {
        match self {
            JsonValue::Array(val) => Some(val),
            _ => None,
        }
    }
    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }
    pub fn as_object(&self) -> Option<&IndexMap<String, Rc<JsonNode>>> {
        match self {
            JsonValue::Object(val) => Some(val),
            _ => None,
        }
    }
}

impl Serialize for JsonValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        match self {
            JsonValue::Null => serializer.serialize_unit(),
            JsonValue::Boolean(b) => serializer.serialize_bool(*b),
            JsonValue::Number(n) => n.serialize(serializer),
            JsonValue::String(s) => serializer.serialize_str(s),
            JsonValue::Array(v) => v.serialize(serializer),
            JsonValue::Object(m) => {
                use serde::ser::SerializeMap;
                let mut map = tri!(serializer.serialize_map(Some(m.len())));
                for (k, v) in m {
                    tri!(map.serialize_entry(k, v));
                }
                map.end()
            }
        }
    }
}

impl Display for JsonValue {
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