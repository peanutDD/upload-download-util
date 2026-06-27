use serde::Serialize;

use crate::utils::AppError;

mod image;
mod pdf;
mod runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrStatus {
    Disabled,
    Unsupported,
    DependencyMissing,
    Failed,
    Completed,
}

impl OcrStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Unsupported => "unsupported",
            Self::DependencyMissing => "dependency_missing",
            Self::Failed => "failed",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrOutcome {
    pub text: String,
    pub status: OcrStatus,
}

#[derive(Debug, Clone)]
pub struct OcrOptions {
    pub enabled: bool,
    pub tesseract_bin: String,
    pub pdftoppm_bin: String,
    pub pdf_max_pages: usize,
}

pub struct OcrExtractor;

impl OcrExtractor {
    pub fn extract(
        data: &[u8],
        mime_type: &str,
        filename: &str,
        enabled: bool,
        tesseract_bin: &str,
    ) -> Result<OcrOutcome, AppError> {
        Self::extract_with_options(
            data,
            mime_type,
            filename,
            OcrOptions {
                enabled,
                tesseract_bin: tesseract_bin.to_string(),
                pdftoppm_bin: "pdftoppm".to_string(),
                pdf_max_pages: 5,
            },
        )
    }

    pub fn extract_with_options(
        data: &[u8],
        mime_type: &str,
        original_filename: &str,
        options: OcrOptions,
    ) -> Result<OcrOutcome, AppError> {
        if !options.enabled {
            return Ok(outcome(OcrStatus::Disabled, ""));
        }
        if !is_ocr_candidate(mime_type, original_filename) {
            return Ok(outcome(OcrStatus::Unsupported, ""));
        }

        if is_pdf(mime_type, original_filename) {
            return pdf::extract_pdf_pages(data, original_filename, &options);
        }

        image::extract_image(data, mime_type, original_filename, &options.tesseract_bin)
    }

    pub fn command_available(bin: &str) -> bool {
        runtime::command_available(bin)
    }
}

pub(super) fn outcome(status: OcrStatus, text: impl Into<String>) -> OcrOutcome {
    OcrOutcome {
        text: text.into(),
        status,
    }
}

fn is_ocr_candidate(mime_type: &str, filename: &str) -> bool {
    mime_type.starts_with("image/")
        || mime_type == "application/pdf"
        || matches!(
            extension(filename).as_deref(),
            Some("png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "webp" | "pdf")
        )
}

fn is_pdf(mime_type: &str, filename: &str) -> bool {
    mime_type == "application/pdf" || matches!(extension(filename).as_deref(), Some("pdf"))
}

pub(super) fn file_suffix(filename: &str, mime_type: &str) -> &'static str {
    match extension(filename).as_deref() {
        Some("jpg" | "jpeg") => ".jpg",
        Some("png") => ".png",
        Some("tif" | "tiff") => ".tif",
        Some("bmp") => ".bmp",
        Some("webp") => ".webp",
        Some("pdf") => ".pdf",
        _ if mime_type == "application/pdf" => ".pdf",
        _ => ".png",
    }
}

fn extension(filename: &str) -> Option<String> {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}
