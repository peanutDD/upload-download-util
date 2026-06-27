use metrics::counter;
use uuid::Uuid;

use crate::{
    services::{
        file_content_extractor::FileContentExtractor,
        fulltext_search::SearchDocument,
        ocr::{OcrExtractor, OcrOptions},
    },
    utils::AppError,
    AppState,
};

pub struct FulltextIndexer<'a> {
    state: &'a AppState,
}

impl<'a> FulltextIndexer<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn index_file(&self, file_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let file = self.state.file_service.get_file(file_id, user_id).await?;
        let data = self.state.file_service.get_file_data(&file).await?;
        let extracted =
            FileContentExtractor::extract_text(&data, &file.mime_type, &file.original_filename)
                .unwrap_or_default();
        let ocr = OcrExtractor::extract_with_options(
            &data,
            &file.mime_type,
            &file.original_filename,
            OcrOptions {
                enabled: self.state.config.search.ocr_enabled,
                tesseract_bin: self.state.config.search.ocr_tesseract_bin.clone(),
                pdftoppm_bin: self.state.config.search.ocr_pdftoppm_bin.clone(),
                pdf_max_pages: self.state.config.search.ocr_pdf_max_pages,
            },
        )?;

        // OCR is best-effort: disabled, unsupported, and missing dependencies
        // are observable statuses, not reasons to fail the indexing worker.
        counter!("fulltext_ocr_status_total", "status" => ocr.status.as_str()).increment(1);
        tracing::info!(
            file_id = %file.id,
            user_id = %file.user_id,
            ocr_status = ocr.status.as_str(),
            "fulltext OCR extraction status"
        );

        let original_filename = file.original_filename.clone();
        self.state.search_index.upsert_document(SearchDocument {
            file_id: file.id,
            user_id: file.user_id,
            filename: original_filename.clone(),
            path: format!("/{original_filename}"),
            extracted_text: extracted,
            ocr_text: ocr.text,
            category: file.category.unwrap_or_default(),
            mime_type: file.mime_type,
        })
    }
}
