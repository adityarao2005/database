use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: Option<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            position: None,
        }
    }

    pub fn with_position(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position: Some(position),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(pos) = self.position {
            write!(f, "Parse error at position {}: {}", pos, self.message)
        } else {
            write!(f, "Parse error: {}", self.message)
        }
    }
}

impl std::error::Error for ParseError {}
