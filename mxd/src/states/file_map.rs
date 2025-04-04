use std::path::Path;

use common::{
    state::{AtomticStateStorage, StateStorage as _},
    utils::{self},
};
use log::error;

#[derive(Debug, Clone)]
pub(crate) struct FileMap {
    pub(crate) file_path: String,
    pub(crate) xxh3: Option<String>,
    pub(crate) sha1: Option<String>,
}
pub(crate) struct FileMapStorage(AtomticStateStorage<String, FileMap>);

impl FileMapStorage {
    pub(crate) fn new() -> Self {
        FileMapStorage(AtomticStateStorage::new())
    }
    pub(crate) async fn add_file_map(
        &self,
        file_path: String,
        publish_name: String,
    ) -> Result<(), String> {
        let path = Path::new(&file_path);
        if !path.exists() {
            return Err(format!("File not found: {}", file_path));
        }
        if !path.is_file() {
            return Err(format!("Path is not a file: {}", file_path));
        }
        let new_path = path
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("Failed to canonicalize path: {}", e))?;
        self.0
            .set(
                publish_name,
                FileMap {
                    file_path: new_path,
                    xxh3: None,
                    sha1: None,
                },
            )
            .await;
        Ok(())
    }

    pub(crate) async fn get_file_map(&self, publish_name: &String) -> Option<FileMap> {
        self.0.get(publish_name).await.map(|f| (*f).clone())
    }

    pub(crate) async fn get_all_files(&self) -> Vec<String> {
        self.0.list().await
    }

    pub(crate) async fn del_file_map(&self, publish_name: &String) {
        self.0.remove(publish_name).await;
    }

    pub(crate) async fn get_file_with_optional_props(
        &self,
        publish_name: &String,
        ensure_xxh3: bool,
        ensure_sha1: bool,
    ) -> Option<FileMap> {
        if self
            .0
            .map_async(publish_name.clone(), async |file_map| {
                let file_map: &FileMap = file_map;
                let mut new_inner: FileMap = file_map.clone();
                if ensure_xxh3 && new_inner.xxh3.is_none() {
                    new_inner.xxh3 = Some(match utils::xxh3_for_file(&new_inner.file_path).await {
                        Ok(hash) => hash,
                        Err(e) => {
                            error!("Failed to calculate xxh3: {}", e);
                            return None;
                        }
                    });
                }
                if ensure_sha1 && new_inner.sha1.is_none() {
                    new_inner.sha1 = Some(match utils::sha1_for_file(&new_inner.file_path).await {
                        Ok(hash) => hash,
                        Err(e) => {
                            error!("Failed to calculate sha1: {}", e);
                            return None;
                        }
                    });
                }
                Some(new_inner)
            })
            .await
        {
            self.0.get(publish_name).await.map(|f| (*f).clone())
        } else {
            None
        }
    }
}
