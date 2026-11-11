//! Safe PDF text extraction.
#![allow(missing_docs)]

use std::path::Path;

use zeroize::Zeroizing;

use crate::error::AppError;

/// Maximum accepted resume size. Larger files are rejected before parsing.
pub const MAX_PDF_BYTES: u64 = 20 * 1024 * 1024;

const PDF_MAGIC: &[u8] = b"%PDF-";

/// Extract text from a PDF file. The returned text is zeroized on drop and must
/// never be persisted or logged.
pub fn extract_text_from_path(path: &Path) -> Result<Zeroizing<String>, AppError> {
    let metadata = std::fs::metadata(path)
        .map_err(|_| AppError::validation("the resume file could not be read"))?;
    if !metadata.is_file() {
        return Err(AppError::validation("the resume path is not a file"));
    }
    if metadata.len() == 0 {
        return Err(AppError::validation("the resume PDF is empty"));
    }
    if metadata.len() > MAX_PDF_BYTES {
        return Err(AppError::validation("the resume PDF is larger than 20 MB"));
    }

    let bytes = std::fs::read(path)
        .map_err(|_| AppError::validation("the resume file could not be read"))?;
    extract_text_from_mem(&bytes)
}

/// Extract text from PDF bytes. Rejects non-PDF, corrupt, encrypted, and
/// image-only files without returning partial content.
pub fn extract_text_from_mem(bytes: &[u8]) -> Result<Zeroizing<String>, AppError> {
    if !bytes.starts_with(PDF_MAGIC) {
        return Err(AppError::validation("the file is not a PDF"));
    }
    if bytes.len() as u64 > MAX_PDF_BYTES {
        return Err(AppError::validation("the resume PDF is larger than 20 MB"));
    }

    let text = pdf_extract::extract_text_from_mem(bytes).map_err(|error| {
        let message = error.to_string().to_ascii_lowercase();
        if message.contains("encrypt") || message.contains("password") {
            AppError::validation("the PDF is encrypted or password-protected")
        } else {
            AppError::validation("the PDF is corrupt or unsupported")
        }
    })?;

    if text.trim().is_empty() {
        return Err(AppError::validation(
            "the PDF contains no extractable text; image-only PDFs are not supported",
        ));
    }

    Ok(Zeroizing::new(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_pdf_bytes() {
        let error = extract_text_from_mem(b"not a pdf").expect_err("reject");
        assert!(error.message().contains("not a PDF"));
    }

    #[test]
    fn rejects_pdf_without_text() {
        let error = extract_text_from_mem(b"%PDF-1.4\nnot really a pdf").expect_err("reject");
        assert!(error.message().contains("corrupt"));
    }
}
