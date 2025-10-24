// Module declarations
pub mod types;
pub mod streaming;
pub mod parser;

// Re-export public interfaces
pub use types::*;
pub use streaming::RenderingControlSubscription;
// Parser methods are implemented as extensions on XmlParser, no direct re-exports needed