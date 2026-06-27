use std::path::PathBuf;

use uuid::Uuid;

use crate::{
    models::{
        file::{File, RenameFileRequest},
        folder::{CreateFolderRequest, Folder, MoveFolderRequest, RenameFolderRequest},
    },
    repositories::{traits::FilesRepository, FoldersRepo, SqlxFilesRepo},
    services::{
        file::{CreateFileFromPathInput, EmbeddingTaskInput, FileService},
        folder::FolderService,
        storage::StorageReadStream,
    },
    utils::validation::sanitize_filename,
    AppState,
};

#[derive(Debug, Clone)]
pub struct WebDavPrincipal {
    pub api_token_id: Uuid,
    pub user_id: Uuid,
    pub read_only: bool,
    pub root_folder_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum DavResource {
    File(File),
    Folder(Folder),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebDavError {
    BadRequest,
    NotFound,
    Conflict,
    Forbidden,
    PreconditionFailed,
    Internal,
}

pub struct WebDavService<'a> {
    state: &'a AppState,
}

pub struct WebDavChildren {
    pub folders: Vec<Folder>,
    pub files: Vec<File>,
}

pub struct WebDavPutInput {
    pub mime_type: String,
    pub file_size: u64,
    pub source_path: PathBuf,
}

pub struct WebDavReadFile {
    pub file: File,
    pub stream: Option<StorageReadStream>,
}

pub enum WebDavMoveOutcome {
    Created,
    NoContent,
}

fn destination_is_same_or_descendant(
    source_segments: &[String],
    destination_segments: &[String],
) -> bool {
    !source_segments.is_empty()
        && destination_segments.len() >= source_segments.len()
        && source_segments
            .iter()
            .zip(destination_segments.iter())
            .all(|(source, destination)| source == destination)
}

impl<'a> WebDavService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn resolve_parent(
        &self,
        principal: &WebDavPrincipal,
        folders: &[String],
    ) -> Result<Option<Uuid>, WebDavError> {
        if folders.is_empty() {
            return Ok(principal.root_folder_id);
        }

        let folder_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            WITH RECURSIVE path_match AS (
                SELECT id, 1 AS depth
                FROM folders
                WHERE user_id = $1
                  AND name = ($3::text[])[1]
                  AND (
                      ($2::uuid IS NULL AND parent_id IS NULL)
                      OR parent_id = $2
                  )

                UNION ALL

                SELECT child.id, path_match.depth + 1
                FROM path_match
                JOIN folders child
                  ON child.parent_id = path_match.id
                 AND child.user_id = $1
                WHERE child.name = ($3::text[])[path_match.depth + 1]
                  AND path_match.depth < cardinality($3::text[])
            )
            SELECT id
            FROM path_match
            WHERE depth = cardinality($3::text[])
            LIMIT 1
            "#,
        )
        .bind(principal.user_id)
        .bind(principal.root_folder_id)
        .bind(folders)
        .fetch_optional(&self.state.pool)
        .await
        .map_err(|_| WebDavError::Internal)?;

        folder_id.map(Some).ok_or(WebDavError::NotFound)
    }

    pub async fn resolve_parent_and_name(
        &self,
        principal: &WebDavPrincipal,
        segments: &[String],
    ) -> Result<(Option<Uuid>, String), WebDavError> {
        let Some(name) = segments.last() else {
            return Err(WebDavError::BadRequest);
        };
        let parent = self
            .resolve_parent(principal, &segments[..segments.len() - 1])
            .await?;
        Ok((parent, name.clone()))
    }

    pub async fn list_children(
        &self,
        user_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<WebDavChildren, WebDavError> {
        let repo = FoldersRepo::new(&self.state.pool);
        let folders = repo
            .list_by_parent(user_id, parent_id)
            .await
            .map_err(|_| WebDavError::Internal)?;
        let files = self
            .state
            .file_service
            .list_by_folder(user_id, parent_id)
            .await
            .map_err(|_| WebDavError::Internal)?;
        Ok(WebDavChildren { folders, files })
    }

    pub async fn create_collection(
        &self,
        principal: &WebDavPrincipal,
        segments: &[String],
    ) -> Result<(), WebDavError> {
        let (parent_id, name) = self.resolve_parent_and_name(principal, segments).await?;
        FolderService::from_state(self.state)
            .create_folder(principal.user_id, CreateFolderRequest { name, parent_id })
            .await
            .map(|_| ())
            .map_err(|_| WebDavError::Conflict)
    }

    pub async fn put_file_from_path(
        &self,
        principal: &WebDavPrincipal,
        segments: &[String],
        input: WebDavPutInput,
    ) -> Result<(), WebDavError> {
        let (folder_id, filename) = self.resolve_parent_and_name(principal, segments).await?;
        let create_input = CreateFileFromPathInput {
            user_id: principal.user_id,
            original_filename: filename,
            mime_type: input.mime_type,
            file_size: input.file_size,
            source_path: &input.source_path,
            content_sha256: None,
            folder_id,
        };
        self.state
            .file_service
            .create_file_from_path(create_input)
            .await
            .map(|_| ())
            .map_err(|_| WebDavError::Conflict)?;
        self.bump_files_cache(principal.user_id).await;
        Ok(())
    }

    pub async fn open_file_for_read(
        &self,
        principal: &WebDavPrincipal,
        segments: &[String],
        range: Option<(u64, u64)>,
        include_body: bool,
    ) -> Result<WebDavReadFile, WebDavError> {
        let (folder_id, filename) = self.resolve_parent_and_name(principal, segments).await?;
        let file = self
            .find_file(principal.user_id, folder_id, &filename)
            .await
            .ok_or(WebDavError::NotFound)?;
        let total = file.file_size.max(0) as u64;
        let stream = if include_body && total > 0 {
            let opened = if let Some((start, end)) = range {
                self.state
                    .file_service
                    .open_file_stream_range(&file, start, end)
                    .await
            } else {
                self.state.file_service.open_file_stream(&file).await
            };
            Some(opened.map_err(|_| WebDavError::NotFound)?)
        } else {
            None
        };
        Ok(WebDavReadFile { file, stream })
    }

    pub async fn find_file(
        &self,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        filename: &str,
    ) -> Option<crate::models::file::File> {
        self.state
            .file_service
            .find_file_by_name_and_folder(user_id, filename, folder_id)
            .await
            .ok()
            .flatten()
    }

    pub async fn find_folder(
        &self,
        user_id: Uuid,
        parent_id: Option<Uuid>,
        name: &str,
    ) -> Option<crate::models::folder::Folder> {
        FoldersRepo::new(&self.state.pool)
            .find_by_name_and_parent(user_id, parent_id, name)
            .await
            .ok()
            .flatten()
    }

    pub async fn find_resource(
        &self,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        name: &str,
    ) -> Option<DavResource> {
        if let Some(file) = self.find_file(user_id, folder_id, name).await {
            return Some(DavResource::File(file));
        }
        self.find_folder(user_id, folder_id, name)
            .await
            .map(DavResource::Folder)
    }

    pub async fn destination_exists(
        &self,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        name: &str,
    ) -> bool {
        self.find_resource(user_id, folder_id, name).await.is_some()
    }

    pub async fn delete_destination(
        &self,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        name: &str,
    ) -> Result<(), WebDavError> {
        match self.find_resource(user_id, folder_id, name).await {
            Some(DavResource::File(file)) => self
                .state
                .file_service
                .delete_file(file.id, user_id)
                .await
                .map_err(|_| WebDavError::Internal),
            Some(DavResource::Folder(folder)) => self.delete_folder_tree(user_id, folder.id).await,
            None => Ok(()),
        }
    }

    pub async fn delete_path(
        &self,
        principal: &WebDavPrincipal,
        segments: &[String],
    ) -> Result<(), WebDavError> {
        let (folder_id, filename) = self.resolve_parent_and_name(principal, segments).await?;
        match self
            .find_resource(principal.user_id, folder_id, &filename)
            .await
        {
            Some(DavResource::File(file)) => self
                .state
                .file_service
                .delete_file(file.id, principal.user_id)
                .await
                .map_err(|_| WebDavError::Internal)?,
            Some(DavResource::Folder(folder)) => {
                self.delete_folder_tree(principal.user_id, folder.id)
                    .await?
            }
            None => return Err(WebDavError::NotFound),
        }
        self.bump_files_cache(principal.user_id).await;
        Ok(())
    }

    pub async fn move_path(
        &self,
        principal: &WebDavPrincipal,
        source_segments: &[String],
        destination_segments: &[String],
        overwrite: bool,
    ) -> Result<WebDavMoveOutcome, WebDavError> {
        let (source_folder_id, source_name) = self
            .resolve_parent_and_name(principal, source_segments)
            .await?;
        let (dest_folder_id, dest_name) = self
            .resolve_parent_and_name(principal, destination_segments)
            .await
            .map_err(|_| WebDavError::Conflict)?;

        if source_folder_id == dest_folder_id && source_name == dest_name {
            return Ok(WebDavMoveOutcome::NoContent);
        }

        if let Some(file) = self
            .find_file(principal.user_id, source_folder_id, &source_name)
            .await
        {
            if self
                .destination_exists(principal.user_id, dest_folder_id, &dest_name)
                .await
            {
                if !overwrite {
                    return Err(WebDavError::PreconditionFailed);
                }
                self.delete_destination(principal.user_id, dest_folder_id, &dest_name)
                    .await?;
            }
            if dest_name != source_name {
                self.state
                    .file_service
                    .rename_file(
                        principal.user_id,
                        file.id,
                        RenameFileRequest { name: dest_name },
                    )
                    .await
                    .map_err(|_| WebDavError::Conflict)?;
            }
            if dest_folder_id != source_folder_id {
                FoldersRepo::new(&self.state.pool)
                    .move_files_to_folder(principal.user_id, &[file.id], dest_folder_id)
                    .await
                    .map_err(|_| WebDavError::Conflict)?;
            }
            self.bump_files_cache(principal.user_id).await;
            return Ok(WebDavMoveOutcome::Created);
        }

        let folder = self
            .find_folder(principal.user_id, source_folder_id, &source_name)
            .await
            .ok_or(WebDavError::NotFound)?;
        if destination_is_same_or_descendant(source_segments, destination_segments) {
            return Err(WebDavError::Conflict);
        }
        if self
            .destination_exists(principal.user_id, dest_folder_id, &dest_name)
            .await
        {
            if !overwrite {
                return Err(WebDavError::PreconditionFailed);
            }
            self.delete_destination(principal.user_id, dest_folder_id, &dest_name)
                .await?;
        }
        let folder_service = FolderService::from_state(self.state);
        if dest_name != source_name {
            folder_service
                .rename_folder(
                    principal.user_id,
                    folder.id,
                    RenameFolderRequest { name: dest_name },
                )
                .await
                .map_err(|_| WebDavError::Conflict)?;
        }
        if dest_folder_id != source_folder_id {
            folder_service
                .move_folder(
                    principal.user_id,
                    folder.id,
                    MoveFolderRequest {
                        parent_id: dest_folder_id,
                    },
                )
                .await
                .map_err(|_| WebDavError::Conflict)?;
        }
        self.bump_files_cache(principal.user_id).await;
        Ok(WebDavMoveOutcome::Created)
    }

    pub async fn copy_path(
        &self,
        principal: &WebDavPrincipal,
        source_segments: &[String],
        destination_segments: &[String],
        overwrite: bool,
    ) -> Result<(), WebDavError> {
        let (source_folder_id, source_name) = self
            .resolve_parent_and_name(principal, source_segments)
            .await?;
        let (dest_folder_id, dest_name) = self
            .resolve_parent_and_name(principal, destination_segments)
            .await
            .map_err(|_| WebDavError::Conflict)?;

        let source_folder = self
            .find_folder(principal.user_id, source_folder_id, &source_name)
            .await;
        if source_folder.is_some()
            && destination_is_same_or_descendant(source_segments, destination_segments)
        {
            return Err(WebDavError::Conflict);
        }

        if self
            .destination_exists(principal.user_id, dest_folder_id, &dest_name)
            .await
        {
            if !overwrite {
                return Err(WebDavError::PreconditionFailed);
            }
            self.delete_destination(principal.user_id, dest_folder_id, &dest_name)
                .await?;
        }

        if let Some(file) = self
            .find_file(principal.user_id, source_folder_id, &source_name)
            .await
        {
            self.copy_file_to_folder(principal.user_id, &file, dest_folder_id, dest_name)
                .await?;
            self.bump_files_cache(principal.user_id).await;
            return Ok(());
        }

        let folder = source_folder.ok_or(WebDavError::NotFound)?;
        self.copy_folder_tree(principal.user_id, folder.id, dest_folder_id, dest_name)
            .await?;
        self.bump_files_cache(principal.user_id).await;
        Ok(())
    }

    pub async fn delete_folder_tree(
        &self,
        user_id: Uuid,
        folder_id: Uuid,
    ) -> Result<(), WebDavError> {
        let repo = FoldersRepo::new(&self.state.pool);
        let folder_ids = repo
            .get_all_descendant_ids(folder_id, user_id)
            .await
            .map_err(|_| WebDavError::Internal)?;
        let file_ids = repo
            .get_all_file_ids_in_folders(user_id, &folder_ids)
            .await
            .map_err(|_| WebDavError::Internal)?;
        self.state
            .file_service
            .batch_delete(&file_ids, user_id)
            .await
            .map_err(|_| WebDavError::Internal)?;
        repo.delete(&folder_ids, user_id)
            .await
            .map_err(|_| WebDavError::Internal)?;
        Ok(())
    }

    pub async fn copy_file_to_folder(
        &self,
        user_id: Uuid,
        file: &crate::models::file::File,
        folder_id: Option<Uuid>,
        filename: String,
    ) -> Result<(), WebDavError> {
        let file_size = file.file_size.max(0) as u64;
        self.state
            .file_service
            .ensure_can_store_detailed(user_id, &file.mime_type, file_size)
            .await
            .map_err(|_| WebDavError::Conflict)?;

        let file_id = Uuid::new_v4();
        let original_filename = filename;
        let sanitized_filename =
            sanitize_filename(&original_filename).map_err(|_| WebDavError::Conflict)?;
        let storage_filename = format!("{file_id}_{sanitized_filename}");
        let file_path = self
            .state
            .storage
            .copy_file_to_user(&file.file_path, user_id, file_id, &storage_filename)
            .await
            .map_err(|_| WebDavError::Conflict)?;

        let repo =
            SqlxFilesRepo::new_with_replica(self.state.pool.clone(), self.state.read_pool.clone());
        let inserted = repo
            .insert(
                file_id,
                user_id,
                &storage_filename,
                &original_filename,
                &file_path,
                file_size,
                &file.mime_type,
                &self.state.config.storage.backend,
                file.content_sha256.as_deref(),
                folder_id,
            )
            .await;

        let file_record = match inserted {
            Ok(file_record) => file_record,
            Err(_) => {
                let _ = self.state.storage.delete_file(&file_path).await;
                return Err(WebDavError::Conflict);
            }
        };

        if let Some(embedding_service) = &self.state.embedding_service {
            let task = EmbeddingTaskInput {
                embedding_service: embedding_service.clone(),
                storage: self.state.storage.clone(),
                file: file_record,
                mime_type: file.mime_type.clone(),
                original_filename,
                file_id,
                user_id,
                pool: self.state.pool.clone(),
            };
            tokio::spawn(async move {
                FileService::generate_embedding_with_content(task).await;
            });
        }

        self.state
            .file_service
            .enqueue_fulltext_index_task_best_effort(file_id, user_id)
            .await;
        Ok(())
    }

    pub async fn copy_folder_tree(
        &self,
        user_id: Uuid,
        source_folder_id: Uuid,
        destination_parent_id: Option<Uuid>,
        destination_name: String,
    ) -> Result<(), WebDavError> {
        let folder_service = FolderService::from_state(self.state);
        let root = folder_service
            .create_folder(
                user_id,
                CreateFolderRequest {
                    name: destination_name,
                    parent_id: destination_parent_id,
                },
            )
            .await
            .map_err(|_| WebDavError::Conflict)?;

        let repo = FoldersRepo::new(&self.state.pool);
        let mut pending = vec![(source_folder_id, root.id)];
        while let Some((source_id, dest_id)) = pending.pop() {
            let files = self
                .state
                .file_service
                .list_by_folder(user_id, Some(source_id))
                .await
                .map_err(|_| WebDavError::Conflict)?;
            for file in files {
                self.copy_file_to_folder(
                    user_id,
                    &file,
                    Some(dest_id),
                    file.original_filename.clone(),
                )
                .await?;
            }

            let child_folders = repo
                .list_by_parent(user_id, Some(source_id))
                .await
                .map_err(|_| WebDavError::Conflict)?;
            for child in child_folders {
                if child.id == root.id {
                    continue;
                }
                let copied = folder_service
                    .create_folder(
                        user_id,
                        CreateFolderRequest {
                            name: child.name,
                            parent_id: Some(dest_id),
                        },
                    )
                    .await
                    .map_err(|_| WebDavError::Conflict)?;
                pending.push((child.id, copied.id));
            }
        }
        Ok(())
    }

    pub async fn bump_files_cache(&self, user_id: Uuid) {
        if let Some(pool) = &self.state.redis {
            let _ = crate::services::redis::RedisService::new(pool.clone())
                .bump_user_cache_version(user_id)
                .await;
        }
    }
}
