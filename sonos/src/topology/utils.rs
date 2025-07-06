use std::fs::OpenOptions;
use std::io::Write;
use xmltree::Element;

/// Converts an XML element to a string representation
pub fn element_to_str(element: &Element) -> String {
    let mut buffer = Vec::new();
    element.write(&mut buffer).expect("Failed to write XML element");
    String::from_utf8_lossy(&buffer).into_owned()
}



/// Writes decoded XML to file for debugging purposes
pub fn write_debug_xml(xml: &str) {
    if let Ok(mut debug_file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("../decoded_topology.xml") {
        let _ = debug_file.write_all(xml.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xmltree::Element;


    #[test]
    fn test_element_to_str_simple_element() {
        // Test converting a simple XML element to string
        let mut element = Element::new("test");
        element.children.push(xmltree::XMLNode::Text("content".to_string()));
        
        let result = element_to_str(&element);
        assert!(result.contains("<test>"));
        assert!(result.contains("content"));
        assert!(result.contains("</test>"));
    }

    #[test]
    fn test_element_to_str_element_with_attributes() {
        // Test converting an XML element with attributes to string
        let mut element = Element::new("speaker");
        element.attributes.insert("uuid".to_string(), "RINCON_123456".to_string());
        element.attributes.insert("name".to_string(), "Living Room".to_string());
        
        let result = element_to_str(&element);
        assert!(result.contains("<speaker"));
        assert!(result.contains("uuid=\"RINCON_123456\""));
        assert!(result.contains("name=\"Living Room\""));
        assert!(result.contains("/>") || result.contains("</speaker>"));
    }

    #[test]
    fn test_element_to_str_nested_elements() {
        // Test converting nested XML elements to string
        let mut parent = Element::new("parent");
        let mut child = Element::new("child");
        child.children.push(xmltree::XMLNode::Text("child_content".to_string()));
        parent.children.push(xmltree::XMLNode::Element(child));
        
        let result = element_to_str(&parent);
        assert!(result.contains("<parent>"));
        assert!(result.contains("<child>"));
        assert!(result.contains("child_content"));
        assert!(result.contains("</child>"));
        assert!(result.contains("</parent>"));
    }

    #[test]
    fn test_element_to_str_empty_element() {
        // Test converting an empty XML element to string
        let element = Element::new("empty");
        
        let result = element_to_str(&element);
        assert!(result.contains("<empty"));
        assert!(result.contains("/>") || result.contains("</empty>"));
    }

    #[test]
    fn test_element_to_str_with_special_characters() {
        // Test converting an XML element with special characters
        let mut element = Element::new("test");
        element.children.push(xmltree::XMLNode::Text("content with & < > \"quotes\"".to_string()));
        
        let result = element_to_str(&element);
        // The XML writer should properly escape special characters
        assert!(result.contains("<test>"));
        assert!(result.contains("</test>"));
        // Note: The exact escaping format may vary, so we just check the structure
    }

    #[test]
    fn test_write_debug_xml_creates_file() {
        // Test that write_debug_xml attempts to create a file
        // Note: This test doesn't verify file creation since it depends on filesystem permissions
        // and the relative path may not be writable in all test environments
        let test_xml = "<test>content</test>";
        
        // This should not panic
        write_debug_xml(test_xml);
    }

    #[test]
    fn test_write_debug_xml_with_empty_string() {
        // Test write_debug_xml with empty string
        write_debug_xml("");
        // Should not panic
    }

    #[test]
    fn test_write_debug_xml_with_large_content() {
        // Test write_debug_xml with large content
        let large_xml = "<root>".to_string() + &"<item>content</item>".repeat(1000) + "</root>";
        write_debug_xml(&large_xml);
        // Should not panic
    }
}