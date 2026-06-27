use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use serde::Serialize;
use tantivy::{
    collector::TopDocs,
    doc,
    query::{BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{Field, IndexRecordOption, Schema, TantivyDocument, Value, STORED, STRING, TEXT},
    Index, IndexReader, IndexWriter, Term,
};
use uuid::Uuid;

use crate::utils::AppError;

mod snippet;

use snippet::{match_source, snippet};

#[derive(Debug, Clone)]
pub struct SearchDocument {
    pub file_id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub path: String,
    pub extracted_text: String,
    pub ocr_text: String,
    pub category: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub file_id: Uuid,
    pub filename: String,
    pub path: String,
    pub mime_type: String,
    pub score: f32,
    pub snippet: String,
    pub match_source: String,
}

#[derive(Clone, Copy)]
struct SearchFields {
    file_id: Field,
    user_id: Field,
    filename: Field,
    path: Field,
    extracted_text: Field,
    ocr_text: Field,
    category: Field,
    mime_type: Field,
}

pub struct SearchIndexService {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: SearchFields,
}

impl SearchIndexService {
    pub fn open_in_memory() -> Result<Self, AppError> {
        let (schema, fields) = build_schema();
        let index = Index::create_in_ram(schema);
        let reader = index.reader().map_err(to_app_error)?;
        let writer = index.writer(50_000_000).map_err(to_app_error)?;
        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    pub fn open_or_create(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let (schema, fields) = build_schema();
        std::fs::create_dir_all(path.as_ref())
            .map_err(|e| AppError::Storage(format!("create search index dir: {e}")))?;
        let index = Index::open_in_dir(path.as_ref()).or_else(|_| {
            Index::create_in_dir(path.as_ref(), schema.clone()).map_err(to_app_error)
        })?;
        let reader = index.reader().map_err(to_app_error)?;
        let writer = index.writer(50_000_000).map_err(to_app_error)?;
        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    pub fn upsert_document(&self, document: SearchDocument) -> Result<(), AppError> {
        let mut writer = self.writer()?;
        writer.delete_term(Term::from_field_text(
            self.fields.file_id,
            &document.file_id.to_string(),
        ));
        writer
            .add_document(doc!(
                self.fields.file_id => document.file_id.to_string(),
                self.fields.user_id => document.user_id.to_string(),
                self.fields.filename => document.filename,
                self.fields.path => document.path,
                self.fields.extracted_text => document.extracted_text,
                self.fields.ocr_text => document.ocr_text,
                self.fields.category => document.category,
                self.fields.mime_type => document.mime_type,
            ))
            .map_err(to_app_error)?;
        writer.commit().map_err(to_app_error)?;
        self.reader.reload().map_err(to_app_error)?;
        Ok(())
    }

    pub fn remove_document(&self, file_id: Uuid) -> Result<(), AppError> {
        let mut writer = self.writer()?;
        writer.delete_term(Term::from_field_text(
            self.fields.file_id,
            &file_id.to_string(),
        ));
        writer.commit().map_err(to_app_error)?;
        self.reader.reload().map_err(to_app_error)?;
        Ok(())
    }

    pub fn search(
        &self,
        user_id: Uuid,
        query: &str,
        limit: usize,
        folder_prefix: Option<&str>,
        mime_type: Option<&str>,
    ) -> Result<Vec<SearchHit>, AppError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let searcher = self.reader.searcher();
        let parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.filename,
                self.fields.extracted_text,
                self.fields.ocr_text,
                self.fields.category,
                self.fields.path,
            ],
        );
        let parsed = parser.parse_query(query).map_err(to_app_error)?;
        let scoped_query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.user_id, &user_id.to_string()),
                    IndexRecordOption::Basic,
                )),
            ),
            (Occur::Must, parsed),
        ]);
        let top_docs = searcher
            .search(
                &scoped_query,
                &TopDocs::with_limit(limit.saturating_mul(4).max(limit)).order_by_score(),
            )
            .map_err(to_app_error)?;

        let mut hits = Vec::new();
        for (score, addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(addr).map_err(to_app_error)?;
            let path = text_field(&doc, self.fields.path).unwrap_or_default();
            if let Some(prefix) = folder_prefix {
                if !path.starts_with(prefix) {
                    continue;
                }
            }
            let doc_mime = text_field(&doc, self.fields.mime_type).unwrap_or_default();
            if let Some(filter) = mime_type {
                if doc_mime != filter {
                    continue;
                }
            }
            let filename = text_field(&doc, self.fields.filename).unwrap_or_default();
            let extracted = text_field(&doc, self.fields.extracted_text).unwrap_or_default();
            let ocr = text_field(&doc, self.fields.ocr_text).unwrap_or_default();
            let category = text_field(&doc, self.fields.category).unwrap_or_default();
            let Some(file_id) = text_field(&doc, self.fields.file_id)
                .and_then(|value| Uuid::parse_str(&value).ok())
            else {
                continue;
            };
            hits.push(SearchHit {
                file_id,
                filename: filename.clone(),
                path,
                mime_type: doc_mime,
                score,
                snippet: snippet(query, [&filename, &extracted, &ocr, &category]),
                match_source: match_source(query, &filename, &ocr, &category),
            });
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    fn writer(&self) -> Result<MutexGuard<'_, IndexWriter>, AppError> {
        self.writer
            .lock()
            .map_err(|e| AppError::File(format!("search index writer lock poisoned: {e}")))
    }
}

fn build_schema() -> (Schema, SearchFields) {
    let mut builder = Schema::builder();
    let file_id = builder.add_text_field("file_id", STRING | STORED);
    let user_id = builder.add_text_field("user_id", STRING | STORED);
    let filename = builder.add_text_field("filename", TEXT | STORED);
    let path = builder.add_text_field("path", STRING | STORED);
    let extracted_text = builder.add_text_field("extracted_text", TEXT | STORED);
    let ocr_text = builder.add_text_field("ocr_text", TEXT | STORED);
    let category = builder.add_text_field("category", TEXT | STORED);
    let mime_type = builder.add_text_field("mime_type", STRING | STORED);
    (
        builder.build(),
        SearchFields {
            file_id,
            user_id,
            filename,
            path,
            extracted_text,
            ocr_text,
            category,
            mime_type,
        },
    )
}

fn text_field(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn to_app_error(error: impl std::fmt::Display) -> AppError {
    AppError::File(format!("search index error: {error}"))
}
