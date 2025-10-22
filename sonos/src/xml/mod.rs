use quick_xml::{events::Event, Reader};

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

/// Core XML parser that wraps quick-xml::Reader and provides high-level parsing methods
pub struct XmlParser<'a> {
    reader: Reader<&'a [u8]>,
    buffer: Vec<u8>,
}

impl<'a> XmlParser<'a> {
    /// Create a new XML parser from a string
    pub fn new(xml: &'a str) -> Self {
        let reader = Reader::from_str(xml);

        Self {
            reader,
            buffer: Vec::new(),
        }
    }

    /// Find a specific element and return its content
    pub fn find_element(&mut self, element_name: &str) -> XmlParseResult<Option<String>> {
        self.buffer.clear();

        loop {
            match self.reader.read_event_into(&mut self.buffer)? {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    if e.name().as_ref() == element_name.as_bytes() {
                        // Found the element, now read its content
                        self.buffer.clear();
                        match self.reader.read_event_into(&mut self.buffer)? {
                            Event::Text(text) => {
                                let content = text.unescape()?.into_owned();
                                return Ok(Some(content));
                            }
                            Event::CData(cdata) => {
                                let content = String::from_utf8_lossy(&cdata).into_owned();
                                return Ok(Some(content));
                            }
                            Event::End(_) => return Ok(Some(String::new())),
                            _ => continue,
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(None)
    }

    /// Find all elements with a specific name and return their content
    pub fn find_all_elements(&mut self, element_name: &str) -> XmlParseResult<Vec<String>> {
        let mut elements = Vec::new();
        self.buffer.clear();

        loop {
            match self.reader.read_event_into(&mut self.buffer)? {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    if e.name().as_ref() == element_name.as_bytes() {
                        // Found the element, now read its content
                        self.buffer.clear();
                        match self.reader.read_event_into(&mut self.buffer)? {
                            Event::Text(text) => {
                                let content = text.unescape()?.into_owned();
                                elements.push(content);
                            }
                            Event::CData(cdata) => {
                                let content = String::from_utf8_lossy(&cdata).into_owned();
                                elements.push(content);
                            }
                            Event::End(_) => elements.push(String::new()),
                            _ => {}
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(elements)
    }

    /// Extract an attribute value from an XML element string
    pub fn extract_attribute(
        &mut self,
        element_xml: &str,
        attr_name: &str,
    ) -> XmlParseResult<String> {
        let mut reader = Reader::from_str(element_xml);
        let mut buf = Vec::new();

        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) | Event::Empty(ref e) => {
                for attr in e.attributes() {
                    let attr = attr?;
                    if attr.key.as_ref() == attr_name.as_bytes() {
                        let value = attr.unescape_value()?.into_owned();
                        return Ok(value);
                    }
                }

                Err(XmlParseError::MissingAttribute {
                    element: element_xml.to_string(),
                    attribute: attr_name.to_string(),
                })
            }
            _ => Err(XmlParseError::InvalidStructure(
                "Expected XML element".to_string(),
            )),
        }
    }

    /// Decode XML entities in text content
    pub fn decode_entities(text: &str) -> String {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&#39;", "'")
            .replace("&#34;", "\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parser_creation() {
        let xml = "<root><element>value</element></root>";
        let _parser = XmlParser::new(xml);
        // Just verify it doesn't panic
        assert!(true);
    }

    #[test]
    fn test_decode_entities() {
        let text = "&lt;test&gt; &amp; &quot;quoted&quot; &apos;single&apos;";
        let decoded = XmlParser::decode_entities(text);
        assert_eq!(decoded, "<test> & \"quoted\" 'single'");
    }
}
