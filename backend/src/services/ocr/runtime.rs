use std::{io::ErrorKind, path::Path, process::Command};

use crate::{
    services::ocr::{outcome, OcrOutcome, OcrStatus},
    utils::AppError,
};

const TESSERACT_LANGUAGE: &str = "eng";

pub(super) fn command_available(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub(super) fn run_tesseract(input: &Path, tesseract_bin: &str) -> Result<OcrOutcome, AppError> {
    // Runtime v1 intentionally uses English OCR only; adding multi-language
    // support requires installing language packs and exposing a config knob.
    let output = Command::new(tesseract_bin)
        .arg(input)
        .arg("stdout")
        .arg("-l")
        .arg(TESSERACT_LANGUAGE)
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(outcome(
            OcrStatus::Completed,
            String::from_utf8_lossy(&output.stdout).trim(),
        )),
        Ok(output) => {
            tracing::warn!(
                status = ?output.status.code(),
                stderr = %String::from_utf8_lossy(&output.stderr),
                "OCR command failed"
            );
            Ok(outcome(OcrStatus::Failed, ""))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            tracing::warn!(tesseract_bin, "OCR dependency missing");
            Ok(outcome(OcrStatus::DependencyMissing, ""))
        }
        Err(error) => Err(AppError::File(format!("运行 OCR 失败: {error}"))),
    }
}
