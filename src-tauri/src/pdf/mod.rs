//! PDF text extraction for the profile pipeline.

pub mod extract;

pub use extract::{MAX_PDF_BYTES, extract_text_from_mem, extract_text_from_path};
