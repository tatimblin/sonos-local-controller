use crate::streaming::subscription::SubscriptionError;

/// Result type for XML parsing operations
pub type XmlParseResult<T> = Result<T, XmlParseError>;

/// Comprehensive error type for XML parsing failures
#[derive(Debug, Clone, thiserror::Error)]
pub enum XmlParseError {
    #[error("XML syntax error: {0}")]
    SyntaxError(String),

    #[error("Missing required element: {element}")]
    MissingElement { element: String },

    #[error("Missing required attribute: {attribute} in element: {element}")]
    MissingAttribute { element: String, attribute: String },

    #[error("Invalid XML structure: {0}")]
    InvalidStructure(String),

    #[error("Entity decoding failed: {0}")]
    EntityDecodingError(String),

    #[error("IO error during parsing: {0}")]
    IoError(String),
}

/// Convert from quick_xml::Error to XmlParseError
impl From<quick_xml::Error> for XmlParseError {
    fn from(error: quick_xml::Error) -> Self {
        match error {
            quick_xml::Error::Io(io_error) => XmlParseError::IoError(io_error.to_string()),
            _ => XmlParseError::SyntaxError(error.to_string()),
        }
    }
}

/// Convert from quick_xml::events::attributes::AttrError to XmlParseError
impl From<quick_xml::events::attributes::AttrError> for XmlParseError {
    fn from(error: quick_xml::events::attributes::AttrError) -> Self {
        XmlParseError::SyntaxError(error.to_string())
    }
}

/// Convert from XmlParseError to SubscriptionError
impl From<XmlParseError> for SubscriptionError {
    fn from(error: XmlParseError) -> Self {
        SubscriptionError::XmlParseError(error.to_string())
    }
}