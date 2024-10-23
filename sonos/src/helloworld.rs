pub struct HelloWorld {
    pub name: String,
}

impl HelloWorld {
    pub fn greeting(&self) -> String {
        format!("Hello {}", self.name)
    }
}
