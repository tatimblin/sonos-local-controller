#[derive(Debug)]
pub enum Page {
    Unknown,
    Startup,
    Control,
}

impl Default for Page {
    fn default() -> Self {
        Page::Startup
    }
}
