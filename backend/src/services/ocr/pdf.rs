use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    services::ocr::{outcome, runtime, OcrOptions, OcrOutcome, OcrStatus},
    utils::AppError,
};

pub(super) fn extract_pdf_pages(
    data: &[u8],
    original_filename: &str,
    options: &OcrOptions,
) -> Result<OcrOutcome, AppError> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| AppError::File(format!("创建 PDF OCR 临时目录失败: {e}")))?;
    let pdf_path = temp_dir
        .path()
        .join(file_name_or_default(original_filename));
    fs::write(&pdf_path, data)
        .map_err(|e| AppError::File(format!("写入 PDF OCR 临时文件失败: {e}")))?;
    let output_prefix = temp_dir.path().join("page");
    let page_limit = options.pdf_max_pages.max(1);

    // Scanned PDFs are rendered page-by-page through Poppler before Tesseract;
    // OCR cost is bounded by OCR_PDF_MAX_PAGES.
    let output = Command::new(&options.pdftoppm_bin)
        .arg("-png")
        .arg("-r")
        .arg("200")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg(page_limit.to_string())
        .arg(&pdf_path)
        .arg(&output_prefix)
        .output();

    match output {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            tracing::warn!(
                status = ?output.status.code(),
                stderr = %String::from_utf8_lossy(&output.stderr),
                "PDF OCR page conversion failed"
            );
            return Ok(outcome(OcrStatus::Failed, ""));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            tracing::warn!(pdftoppm_bin = %options.pdftoppm_bin, "PDF OCR dependency missing");
            return Ok(outcome(OcrStatus::DependencyMissing, ""));
        }
        Err(error) => return Err(AppError::File(format!("运行 PDF OCR 转换失败: {error}"))),
    }

    let mut pages = generated_png_pages(temp_dir.path())?;
    pages.sort();
    pages.truncate(page_limit);
    if pages.is_empty() {
        return Ok(outcome(OcrStatus::Failed, ""));
    }

    let mut text = Vec::new();
    let mut saw_dependency_missing = false;
    for page in pages {
        match runtime::run_tesseract(&page, &options.tesseract_bin)? {
            OcrOutcome {
                status: OcrStatus::Completed,
                text: page_text,
            } if !page_text.is_empty() => text.push(page_text),
            OcrOutcome {
                status: OcrStatus::DependencyMissing,
                ..
            } => saw_dependency_missing = true,
            _ => {}
        }
    }

    if saw_dependency_missing {
        return Ok(outcome(OcrStatus::DependencyMissing, ""));
    }
    if text.is_empty() {
        return Ok(outcome(OcrStatus::Failed, ""));
    }
    Ok(outcome(OcrStatus::Completed, text.join("\n")))
}

fn generated_png_pages(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let entries =
        fs::read_dir(dir).map_err(|e| AppError::File(format!("读取 PDF OCR 页面失败: {e}")))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .collect())
}

fn file_name_or_default(filename: &str) -> String {
    Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("scan.pdf")
        .to_string()
}
