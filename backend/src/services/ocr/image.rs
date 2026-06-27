use std::io::Write;

use crate::{
    services::ocr::{file_suffix, runtime, OcrOutcome},
    utils::AppError,
};

pub(super) fn extract_image(
    data: &[u8],
    mime_type: &str,
    original_filename: &str,
    tesseract_bin: &str,
) -> Result<OcrOutcome, AppError> {
    let mut input = tempfile::Builder::new()
        .suffix(file_suffix(original_filename, mime_type))
        .tempfile()
        .map_err(|e| AppError::File(format!("创建 OCR 临时文件失败: {e}")))?;
    input
        .write_all(data)
        .map_err(|e| AppError::File(format!("写入 OCR 临时文件失败: {e}")))?;
    input
        .flush()
        .map_err(|e| AppError::File(format!("刷新 OCR 临时文件失败: {e}")))?;

    runtime::run_tesseract(input.path(), tesseract_bin)
}
