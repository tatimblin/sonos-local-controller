use std::net::UdpSocket;
use crate::util::ssdp::send_ssdp_request;

pub fn search() -> std::io::Result<()> {
  let mut socket = UdpSocket::bind("0.0.0.0:0")?;
  let responses = send_ssdp_request(
      &mut socket,
      "239.255.255.250:1900",
      "urn:schemas-upnp-org:device:ZonePlayer:1"
  )?;

  for response in responses {
    match response {
      Ok(ssdp_response) => {
        println!("Device found at: {}", ssdp_response.location);
      }
      Err(e) => {
        println!("Error receiving SSDP response: {}", e);
      }
    }
  }

  Ok(())
}
