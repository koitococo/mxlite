use std::path::Path;

use crate::utils::{
  hash,
  states::{StateMap, States as _},
};

#[derive(Debug, Clone)]
pub struct FileMap {
  pub file_path: String,
  pub xxh3: Option<String>,
  pub md5: Option<String>,
  pub sha1: Option<String>,
  pub sha256: Option<String>,
  pub sha512: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MapItem {
  File(FileMap),
  Dir(String),
}

pub type FileMapStorage = StateMap<String, MapItem>;

impl FileMapStorage {
  pub fn add_file_map(&self, file_path: String, publish_name: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
      return Err(format!("File not found: {file_path}"));
    }
    if !path.is_file() {
      return Err(format!("Path is not a file: {file_path}"));
    }
    let new_path = path
      .canonicalize()
      .map(|p| p.to_string_lossy().to_string())
      .map_err(|e| format!("Failed to canonicalize path: {e}"))?;
    self.insert(
      publish_name,
      MapItem::File(FileMap {
        file_path: new_path,
        xxh3: None,
        md5: None,
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
      return Err(format!("Directory not found: {dir_path}"));
    }
    if !path.is_dir() {
      return Err(format!("Path is not a directory: {dir_path}"));
    }
    let new_path = path
      .canonicalize()
      .map(|p| p.to_string_lossy().to_string())
      .map_err(|e| format!("Failed to canonicalize path: {e}"))?;
    self.insert(publish_name, MapItem::Dir(new_path));
    Ok(())
  }

  pub(crate) async fn get_file_with_optional_props(
    &self, publish_name: &String, ensure_xxh3: bool, ensure_md5: bool, ensure_sha1: bool, ensure_sha256: bool,
    ensure_sha512: bool,
  ) -> Option<FileMap> {
    if let Some(file_map) = self.get_arc(publish_name) &&
      let MapItem::File(mut new_inner) = (*file_map).clone()
    {
      if ensure_xxh3 &&
        new_inner.xxh3.is_none() &&
        let Ok(hash) = hash::xxh3_for_file(&new_inner.file_path).await
      {
        new_inner.xxh3 = Some(hash);
      }
      if ensure_md5 || ensure_sha1 || ensure_sha256 || ensure_sha512 {
        let calc_md5 = ensure_md5 && new_inner.sha1.is_none();
        let calc_sha1 = ensure_sha1 && new_inner.sha1.is_none();
        let calc_sha256 = ensure_sha256 && new_inner.sha256.is_none();
        let calc_sha512 = ensure_sha512 && new_inner.sha512.is_none();
        if let Ok((md5, sha1, sha256, sha512)) =
          hash::hashes_for_file(&new_inner.file_path, calc_md5, calc_sha1, calc_sha256, calc_sha512).await
        {
          if calc_md5 {
            new_inner.md5 = md5;
          }
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
      self.insert(publish_name.clone(), MapItem::File(new_inner.clone()));
      return Some(new_inner);
    }
    None
  }

  pub(crate) fn get_dir_child_path(&self, publish_name: &String, subpath: &String) -> Option<String> {
    if let Some(map_item) = self.get_arc(publish_name) &&
      let MapItem::Dir(path) = (*map_item).clone()
    {
      let path = Path::new(&path);
      let new_path = path.join(subpath);
      if new_path.exists() {
        return Some(new_path.to_string_lossy().to_string());
      }
    }
    None
  }
}
