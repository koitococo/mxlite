use std::path::Path;

use common::{
  state::{AtomticStateStorage, StateStorage as _},
  utils::{self},
};

#[derive(Debug, Clone)]
pub(crate) struct FileMap {
  pub(crate) file_path: String,
  pub(crate) xxh3: Option<String>,
  pub(crate) sha1: Option<String>,
  pub(crate) sha256: Option<String>,
  pub(crate) sha512: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum MapItem {
  File(FileMap),
  Dir(String),
}

pub(crate) struct FileMapStorage(AtomticStateStorage<String, MapItem>);

impl FileMapStorage {
  pub(crate) fn new() -> Self { FileMapStorage(AtomticStateStorage::new()) }

  pub(crate) fn add_file_map(&self, file_path: String, publish_name: String) -> Result<(), String> {
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
    self.0.insert(
      publish_name,
      MapItem::File(FileMap {
        file_path: new_path,
        xxh3: None,
        sha1: None,
        sha256: None,
        sha512: None,
      }),
    );
    Ok(())
  }

  pub(crate) fn add_dir_map(&self, dir_path: String, publish_name: String) -> Result<(), String> {
    let path = Path::new(&dir_path);
    if !path.exists() {
      return Err(format!("Directory not found: {}", dir_path));
    }
    if !path.is_dir() {
      return Err(format!("Path is not a directory: {}", dir_path));
    }
    let new_path = path
      .canonicalize()
      .map(|p| p.to_string_lossy().to_string())
      .map_err(|e| format!("Failed to canonicalize path: {}", e))?;
    self.0.insert(publish_name, MapItem::Dir(new_path));
    Ok(())
  }

  pub(crate) fn list_map(&self) -> Vec<String> { self.0.list() }

  pub(crate) fn del_map(&self, publish_name: &String) { self.0.remove(publish_name); }

  pub(crate) async fn get_file_with_optional_props(
    &self, publish_name: &String, ensure_xxh3: bool, ensure_sha1: bool, ensure_sha256: bool, ensure_sha512: bool,
  ) -> Option<FileMap> {
    if let Some(file_map) = self.0.get(publish_name) {
      if let MapItem::File(mut new_inner) = (*file_map).clone() {
        if ensure_xxh3 && new_inner.xxh3.is_none() {
          if let Ok(hash) = utils::xxh3_for_file(&new_inner.file_path).await {
            new_inner.xxh3 = Some(hash);
          }
        }
        if ensure_sha1 || ensure_sha256 || ensure_sha512 {
          let calc_sha1 = ensure_sha1 && new_inner.sha1.is_none();
          let calc_sha256 = ensure_sha256 && new_inner.sha256.is_none();
          let calc_sha512 = ensure_sha512 && new_inner.sha512.is_none();
          if let Ok((sha1, sha256, sha512)) = utils::sha_for_file(&new_inner.file_path, calc_sha1, calc_sha256, calc_sha512).await {
            if calc_sha1 {
              new_inner.sha1 = sha1;
            }
            if calc_sha256 {
              new_inner.sha256 = sha256;
            }
            if calc_sha512 {
              new_inner.sha512 = sha512;
            }
          }
        }
        self.0.insert(publish_name.clone(), MapItem::File(new_inner.clone()));
        return Some(new_inner);
      }
    }
    None
  }

  pub(crate) fn get_dir_child_path(&self, publish_name: &String, subpath: &String) -> Option<String> {
    if let Some(map_item) = self.0.get(publish_name) {
      if let MapItem::Dir(path) = (*map_item).clone() {
        let path = Path::new(&path);
        let new_path = path.join(subpath);
        if new_path.exists() {
          return Some(new_path.to_string_lossy().to_string());
        }
      }
    }
    None
  }
}
