//! Input parsing and data source handling.

mod parser;
mod source;

pub use parser::{Parser, ParserConfig};
pub use source::{DataTable, SourceMetadata};
