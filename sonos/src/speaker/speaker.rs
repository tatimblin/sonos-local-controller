use ureq::Error;

pub struct Speaker {
  pub name: String,
}

impl Speaker {
  pub fn from_ip(ip: String) -> Result<Speaker, String> {
    match ureq::get(&ip).call() {
      Ok(response) => {
        println!("{:?}", response.into_string());
      },
      Err(Error::Status(code, response)) => {
        println!("{} {:?}", code, response);
      },
      Err(_) => {
        println!("Failed to make connection with device");
      }
    }

    Ok(Speaker {
      name: "Tristan's Speaker".to_string(),
    })
  }
}
