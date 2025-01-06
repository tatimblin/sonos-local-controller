#[derive(Debug)]
pub enum Page {
    Startup,
    Control,
}

impl Default for Page {
    fn default() -> Self {
        Page::Startup
    }
}
