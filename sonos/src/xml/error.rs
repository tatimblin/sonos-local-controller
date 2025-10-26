use crate::error::SonosError;

#[derive(Debug)]
pub enum XmlParseError {
    SyntaxError(String),
    InvalidFormat(String),
    MissingElement(String),
}

impl std::fmt::Display for XmlParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlParseError::SyntaxError(msg) => write!(f, "XML syntax error: {}", msg),
            XmlParseError::InvalidFormat(msg) => write!(f, "Invalid XML format: {}", msg),
            XmlParseError::MissingElement(msg) => write!(f, "Missing XML element: {}", msg),
        }
    }
}

impl std::error::Error for XmlParseError {}

impl From<XmlParseError> for SonosError {
    fn from(err: XmlParseError) -> Self {
        SonosError::ParseError(err.to_string())
    }
}

pub type XmlParseResult<T> = std::result::Result<T, XmlParseError>;