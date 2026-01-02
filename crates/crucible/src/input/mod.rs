//! Input parsing and data source handling.

mod context;
mod parser;
mod source;

pub use context::ContextHints;
pub use parser::{Parser, ParserConfig};
pub use source::{DataTable, SourceMetadata};
