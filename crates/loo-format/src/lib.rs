//! Reader and writer for Looping (`.loo`) documents.
//!
//! The format is an MFC `CArchive` stream produced by Looping 4.1's
//! `CLoopingDoc::Serialize`; see `docs/FORMAT.md` for the layout.

pub mod archive;
pub mod document;
pub mod objects;
pub mod settings;

pub use document::{Document, View, CURRENT_VERSION};
pub use objects::*;
pub use settings::Settings;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unexpected end of file at {at:#x} (wanted {wanted} bytes)")]
    Eof { at: usize, wanted: usize },
    #[error("unknown class tag {tag} at {at:#x}")]
    BadClassTag { at: usize, tag: u32 },
    #[error("unsupported class {class} at {at:#x}")]
    UnknownClass { at: usize, class: String },
    #[error("unexpected {what} at {at:#x}")]
    Unexpected { at: usize, what: String },
    #[error("{len} trailing bytes at {at:#x}")]
    TrailingData { at: usize, len: usize },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Document {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        Document::load(&std::fs::read(path)?)
    }
}
