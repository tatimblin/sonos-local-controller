use crate::error::Result;

pub struct SoapRequest {
  pub service_type: String,
  pub action: String,
  pub params: Vec<(String, String)>,
}

pub struct SoapResponse {
  pub body: String,
}

pub struct SoapClient {
  http_client: reqwest::blocking::Client,
  _timeout: std::time::Duration,
}

impl SoapClient {
  pub fn new(timeout: std::time::Duration) -> Result<Self> {
    let http_client = reqwest::blocking::Client::builder()
      .timeout(timeout)
      .build()
      .map_err(|e| crate::error::SonosError::CommunicationError(e.to_string()))?;

    Ok(Self {
      http_client,
      _timeout: timeout,
    })
  }

  pub fn call(&self, device_url: &str, service_path: &str, request: SoapRequest) -> Result<SoapResponse> {
    let url = format!("{}{}", device_url, service_path);
    let body = Self::build_soap_envelope(&request);

    let response = self
      .http_client
      .post(&url)
      .header("Content-Type", "text/xml; charset=\"utf-8\"")
      .header(
        "SOAPACTION",
        format!("{}#{}", request.service_type, request.action),
      )
      .body(body)
      .send()
      .map_err(|e| crate::error::SonosError::CommunicationError(e.to_string()))?;

    let response_body = response
      .text()
      .map_err(|e| crate::error::SonosError::CommunicationError(e.to_string()))?;

    if response_body.contains("faultstring") {
      let fault = Self::extract_fault_string(&response_body);
      return Err(crate::error::SonosError::SoapFault(fault));
    }

    Ok(SoapResponse { body: response_body })
  }

  fn build_soap_envelope(request: &SoapRequest) -> String {
    let mut params_xml = String::new();
    for (key, value) in &request.params {
      params_xml.push_str(&format!("<{}>{}</{}>\n", key, value, key));
    }

    format!(
      "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
        <s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" \
        s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">\n\
        <s:Body>\n\
        <u:{} xmlns:u=\"{}\">\n\
        {}\
        </u:{}>\n\
        </s:Body>\n\
        </s:Envelope>",
      request.action, request.service_type, params_xml, request.action
    )
  }

  fn extract_fault_string(xml: &str) -> String {
    xml.split("<faultstring>")
      .nth(1)
      .and_then(|s| s.split("</faultstring>").next())
      .unwrap_or("Unknown SOAP fault")
      .to_string()
  }

  pub fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    xml.find(&start_tag)
      .and_then(|start| {
        let content_start = start + start_tag.len();
        xml[content_start..].find(&end_tag).map(|end| {
          xml[content_start..content_start + end].to_string()
        })
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_soap_request_creation() {
    let request = SoapRequest {
      service_type: "urn:schemas-upnp-org:service:AVTransport:1".to_string(),
      action: "Play".to_string(),
      params: vec![
        ("InstanceID".to_string(), "0".to_string()),
        ("Speed".to_string(), "1".to_string()),
      ],
    };

    assert_eq!(request.action, "Play");
    assert_eq!(request.params.len(), 2);
  }

  #[test]
  fn test_build_soap_envelope() {
    let request = SoapRequest {
      service_type: "urn:schemas-upnp-org:service:AVTransport:1".to_string(),
      action: "SetVolume".to_string(),
      params: vec![
        ("InstanceID".to_string(), "0".to_string()),
        ("Channel".to_string(), "Master".to_string()),
        ("DesiredVolume".to_string(), "50".to_string()),
      ],
    };

    let envelope = SoapClient::build_soap_envelope(&request);

    assert!(envelope.contains("<?xml version=\"1.0\""));
    assert!(envelope.contains("<u:SetVolume"));
    assert!(envelope.contains("<InstanceID>0</InstanceID>"));
    assert!(envelope.contains("<Channel>Master</Channel>"));
    assert!(envelope.contains("<DesiredVolume>50</DesiredVolume>"));
    assert!(envelope.contains("</u:SetVolume>"));
  }

  #[test]
  fn test_extract_fault_string() {
    let fault_response = r#"<?xml version="1.0"?>
      <s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
        <s:Body>
          <s:Fault>
            <faultcode>s:Client</faultcode>
            <faultstring>Invalid Volume</faultstring>
          </s:Fault>
        </s:Body>
      </s:Envelope>"#;

    let fault = SoapClient::extract_fault_string(fault_response);

    assert_eq!(fault, "Invalid Volume");
  }

  #[test]
  fn test_extract_fault_string_unknown() {
    let fault_response = r#"<response>No fault here</response>"#;

    let fault = SoapClient::extract_fault_string(fault_response);

    assert_eq!(fault, "Unknown SOAP fault");
  }

  #[test]
  fn test_extract_xml_value() {
    let xml = "<response><volume>75</volume><muted>1</muted></response>";

    let volume = SoapClient::extract_xml_value(xml, "volume");
    assert_eq!(volume, Some("75".to_string()));

    let muted = SoapClient::extract_xml_value(xml, "muted");
    assert_eq!(muted, Some("1".to_string()));

    let missing = SoapClient::extract_xml_value(xml, "missing");
    assert_eq!(missing, None);
  }

  #[test]
  fn test_extract_xml_value_nested() {
    let xml = r#"<response>
        <u:GetVolumeResponse xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1">
          <CurrentVolume>50</CurrentVolume>
        </u:GetVolumeResponse>
      </response>"#;

    let volume = SoapClient::extract_xml_value(xml, "CurrentVolume");
    assert_eq!(volume, Some("50".to_string()));
  }

  #[test]
  fn test_soap_envelope_header() {
    let request = SoapRequest {
      service_type: "urn:schemas-upnp-org:service:AVTransport:1".to_string(),
      action: "Play".to_string(),
      params: vec![
        ("InstanceID".to_string(), "0".to_string()),
        ("Speed".to_string(), "1".to_string()),
      ],
    };

    let envelope = SoapClient::build_soap_envelope(&request);

    assert!(envelope.contains("xmlns:u=\"urn:schemas-upnp-org:service:AVTransport:1\""));
    assert!(envelope.contains("<u:Play"));
    assert!(envelope.contains("</u:Play>"));
  }
}
