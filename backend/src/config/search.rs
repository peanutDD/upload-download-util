use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    pub huggingface_api_token: Option<String>,
    pub huggingface_model_id: String,
    pub huggingface_api_url: String,
    pub fulltext_search_enabled: bool,
    pub fulltext_index_path: String,
    pub ocr_enabled: bool,
    pub ocr_tesseract_bin: String,
    pub ocr_pdftoppm_bin: String,
    pub ocr_pdf_max_pages: usize,
}
