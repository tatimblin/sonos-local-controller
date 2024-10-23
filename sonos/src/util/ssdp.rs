use std::net::UdpSocket;
use std::time::Duration;
use std::str;

/// Sends an SSDP M-SEARCH request and returns the responses as a vector of strings.
pub fn send_ssdp_request(host: &str, target: &str) -> std::io::Result<Vec<String>> {
    // Create a UDP socket bound to any local address and port
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    // Allow the socket to send and receive multicast packets
    socket.set_multicast_loop_v4(true)?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;

    // SSDP M-SEARCH request
    let m_search = format!(
      "M-SEARCH * HTTP/1.1\r\n\
      HOST: {}\r\n\
      MAN: \"ssdp:discover\"\r\n\
      MX: 2\r\n\
      ST: {}\r\n\
      USER-AGENT: Rust/1.0 UPnP/1.0 MyClient/1.0\r\n\
      \r\n",
      host,
      target
    );

    // Send the M-SEARCH request
    socket.send_to(m_search.as_bytes(), host)?;

    let mut responses = Vec::new();
    let mut buf = [0; 1024];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, _)) => {
                let response = str::from_utf8(&buf[..amt]).expect("Failed to parse response");
                responses.push(response.to_string());
            }
            Err(e) => {
                // Break the loop if no more responses or an error occurs
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    break;  // Timed out, no more responses
                } else {
                    println!("Error receiving SSDP response: {}", e);
                }
            }
        }
    }

    Ok(responses)
}
