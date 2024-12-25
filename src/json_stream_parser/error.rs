#[derive(Debug)]
pub struct ParseError {
    pub msg: String
}

impl ParseError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into()
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct LogicalError {
    msg: String
}

impl LogicalError {
    #[allow(dead_code)]
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into()
        }
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        Self {
            msg: "String is not in UTF8".to_string()
        }
    }
}
impl From<std::num::ParseFloatError> for ParseError {
    fn from(_: std::num::ParseFloatError) -> Self {
        Self {
            msg: "Float parse error".to_string()
        }
    }
}
impl From<std::num::ParseIntError> for ParseError {
    fn from(_: std::num::ParseIntError) -> Self {
        Self {
            msg: "Int parse error".to_string()
        }
    }
}