use std::net::UdpSocket;
use std::time::Duration;
use std::str;

pub trait UdpSocketTrait {
    fn send_to(&mut self, buf: &[u8], addr: &str) -> std::io::Result<usize>;
    fn recv_from(&mut self, buf: &mut [u8]) -> std::io::Result<(usize, String)>;
    fn set_multicast_loop_v4(&mut self, multicast_loop: bool) -> std::io::Result<()>;
    fn set_read_timeout(&mut self, dur: Option<Duration>) -> std::io::Result<()>;
}

/// Implement the trait for the standard UdpSocket
impl UdpSocketTrait for UdpSocket {
    fn send_to(&mut self, buf: &[u8], addr: &str) -> std::io::Result<usize> {
        // Call the actual send_to method from UdpSocket
        UdpSocket::send_to(self, buf, addr)
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> std::io::Result<(usize, String)> {
        let (size, src) = UdpSocket::recv_from(self, buf)?;
        Ok((size, src.to_string()))
    }

    fn set_multicast_loop_v4(&mut self, loop_v4: bool) -> std::io::Result<()> {
        UdpSocket::set_multicast_loop_v4(self, loop_v4)
    }
    
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        UdpSocket::set_read_timeout(self, timeout)
    }
}

/// Sends an SSDP M-SEARCH request and returns the responses as a vector of strings.
pub fn send_ssdp_request<S: UdpSocketTrait>(socket: &mut S, host: &str, target: &str) -> std::io::Result<Vec<String>> {
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

pub struct MockUdpSocket {
    responses: Vec<(usize, String)>,
    send_error: Option<Box<dyn std::error::Error>>, // Use Box<dyn Error>
    response_index: usize,
}

impl UdpSocketTrait for MockUdpSocket {
    fn send_to(&mut self, _buf: &[u8], _addr: &str) -> std::io::Result<usize> {
        if let Some(ref error) = self.send_error {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, error.to_string()));
        }
        Ok(0)
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> std::io::Result<(usize, String)> {
        if self.response_index >= self.responses.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock));
        }

        let (size, response) = self.responses[self.response_index].clone();
        buf[..size].copy_from_slice(response.as_bytes());

        self.response_index += 1;

        Ok((size, "mock_address".to_string()))
    }

    fn set_multicast_loop_v4(&mut self, _multicast_loop: bool) -> std::io::Result<()> {
        Ok(())
    }

    fn set_read_timeout(&mut self, _dur: Option<Duration>) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTICAST_ADDRESS: &str = "239.255.255.250:1900";
    const SONOS_SEARCH_TARGET: &str = "urn:schemas-upnp-org:device:ZonePlayer:1";

    #[test]
    fn test_send_ssdp_request_empty_response() {
        let mut mock_socket = MockUdpSocket {
            responses: vec![],
            send_error: None,
            response_index: 0,
        };

        let result = send_ssdp_request(&mut mock_socket, MULTICAST_ADDRESS, SONOS_SEARCH_TARGET);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_send_ssdp_request_send_error() {
        let mut mock_socket = MockUdpSocket {
            responses: vec![],
            send_error: Some(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Send failed"))),
            response_index: 0,
        };

        let result = send_ssdp_request(&mut mock_socket, MULTICAST_ADDRESS, SONOS_SEARCH_TARGET);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn test_send_ssdp_request_success() {
        // Mock response to simulate a device's SSDP response
        let mut mock_socket = MockUdpSocket {
            responses: vec![
                (49, "HTTP/1.1 200 OK\r\nLOCATION: http://mock_device\r\n\r\n".to_string()),
            ],
            send_error: None,
            response_index: 0,
        };

        let result = send_ssdp_request(&mut mock_socket, MULTICAST_ADDRESS, SONOS_SEARCH_TARGET).unwrap();
        
        // Verify the mock response is returned correctly
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "HTTP/1.1 200 OK\r\nLOCATION: http://mock_device\r\n\r\n");
    }
}
