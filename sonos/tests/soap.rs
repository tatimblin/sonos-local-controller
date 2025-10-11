use sonos::transport::soap::{SoapClient, SoapRequest};
use sonos::error::Result;

#[test]
fn test_soap_client_construction() -> Result<()> {
  let _client = SoapClient::new(std::time::Duration::from_secs(5))?;
  Ok(())
}

#[test]
fn test_get_info() {
  let device_url = std::env::var("SONOS_DEVICE_URL")
    .unwrap_or_else(|_| "http://10.0.4.36:1400".to_string());
  let duration = std::time::Duration::from_secs(5);
  let client = SoapClient::new(duration).unwrap();

  let request = SoapRequest {
    service_type: "urn:schemas-upnp-org:service:DeviceProperties:1".to_string(),
    action: "GetZoneInfo".to_string(),
    params: vec![],
  };

  let response = client.call(
    &device_url,
    "/DeviceProperties/Control",
    request,
  )
    .expect("Failed to call GetZoneInfo");

  println!("Response body:\n{}", response.body);

  assert!(response.body.contains("GetZoneInfoResponse"));
  assert!(!response.body.contains("faultstring"));
}
