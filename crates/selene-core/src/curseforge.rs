use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::loader::Loader;
use crate::mods::{CatalogSort, ContentKind, RemoteFile};
use crate::{Result, http};

const API_URL: &str = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID: u32 = 432;
const SHA1_ALGORITHM: u32 = 1;
const REQUIRED_DEPENDENCY: u32 = 3;

/// CurseForge needs an API key for every request.
#[derive(Debug, Clone)]
pub struct CurseForge {
    client: Client,
    api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Response<T> {
    data: T,
    pagination: Option<Pagination>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pagination {
    total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: u64,
    pub logo: Option<Logo>,
    #[serde(default)]
    pub authors: Vec<Author>,
    pub links: Links,
    #[serde(default)]
    pub screenshots: Vec<Screenshot>,
    #[serde(default)]
    pub class_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Logo {
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub website_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub parent_category_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: u32,
    pub mod_id: u32,
    #[serde(default)]
    pub display_name: String,
    pub file_name: String,
    #[serde(default)]
    pub file_date: Option<String>,
    /// 1 is a release, 2 a beta, 3 an alpha.
    #[serde(default)]
    pub release_type: u8,
    /// Missing when the author does not allow other apps to download the file.
    pub download_url: Option<String>,
    pub file_length: u64,
    #[serde(default)]
    pub hashes: Vec<FileHash>,
    #[serde(default)]
    pub dependencies: Vec<FileDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDependency {
    pub mod_id: u32,
    pub relation_type: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintMatches {
    exact_matches: Vec<FingerprintMatch>,
}

#[derive(Debug, Clone, Deserialize)]
struct FingerprintMatch {
    file: FingerprintFile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintFile {
    id: u32,
    mod_id: u32,
    file_fingerprint: u32,
}

/// A CurseForge file recognised by its fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownFile {
    pub mod_id: u32,
    pub file_id: u32,
}

#[derive(Debug, Clone)]
pub struct SearchQuery<'a> {
    pub text: &'a str,
    pub kind: ContentKind,
    pub game_version: &'a str,
    pub loader: Option<Loader>,
    pub categories: &'a [u32],
    pub sort: CatalogSort,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub mods: Vec<Mod>,
    pub total: u32,
}

/// What resolving an install produced: files to download, and mods whose authors only allow
/// downloads from the CurseForge website.
#[derive(Debug, Clone, Default)]
pub struct InstallPlan {
    /// Mod id and the file to download for it.
    pub files: Vec<(u32, RemoteFile)>,
    pub manual: Vec<u32>,
}

fn class_id(kind: ContentKind) -> u32 {
    match kind {
        ContentKind::Mod => 6,
        ContentKind::ResourcePack => 12,
        ContentKind::ShaderPack => 6552,
        ContentKind::DataPack => 6945,
    }
}

/// The kind of content a project of `class_id` is. Data packs need a world, so a pack's data
/// packs are not told apart from mods.
pub fn kind_of_class(class_id: u32) -> ContentKind {
    match class_id {
        12 => ContentKind::ResourcePack,
        6552 => ContentKind::ShaderPack,
        _ => ContentKind::Mod,
    }
}

fn loader_type(loader: Loader) -> u32 {
    match loader {
        Loader::Fabric => 4,
        Loader::Quilt => 5,
        Loader::Forge => 1,
        Loader::NeoForge => 6,
    }
}

impl File {
    fn sha1(&self) -> Option<&str> {
        self.hashes
            .iter()
            .find(|hash| hash.algo == SHA1_ALGORITHM)
            .map(|hash| hash.value.as_str())
    }

    pub fn remote_file(&self) -> Option<RemoteFile> {
        Some(RemoteFile {
            url: self.download_url.clone()?,
            file_name: self.file_name.clone(),
            sha1: self.sha1()?.to_owned(),
            size: self.file_length,
        })
    }
}

impl CurseForge {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self { client, api_key }
    }

    fn get(&self, path: &str) -> RequestBuilder {
        self.request(Method::GET, path)
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let request = self.client.request(method, format!("{API_URL}{path}"));
        match &self.api_key {
            Some(key) => request.header("x-api-key", key),
            None => request,
        }
    }

    pub async fn search(&self, query: &SearchQuery<'_>) -> Result<SearchResults> {
        let sort_field = match query.sort {
            CatalogSort::Relevance => 2,
            CatalogSort::Downloads => 6,
            CatalogSort::Newest => 11,
            CatalogSort::Updated => 3,
        };
        let mut params = vec![
            ("gameId", MINECRAFT_GAME_ID.to_string()),
            ("classId", class_id(query.kind).to_string()),
            ("gameVersion", query.game_version.to_owned()),
            ("searchFilter", query.text.to_owned()),
            ("sortField", sort_field.to_string()),
            ("sortOrder", "desc".to_owned()),
            ("index", query.offset.to_string()),
            ("pageSize", query.limit.to_string()),
        ];
        if let Some(loader) = query.loader.filter(|_| query.kind == ContentKind::Mod) {
            params.push(("modLoaderType", loader_type(loader).to_string()));
        }
        if !query.categories.is_empty() {
            params.push(("categoryIds", serde_json::to_string(query.categories)?));
        }
        let response: Response<Vec<Mod>> =
            http::send(&self.client, self.get("/mods/search").query(&params))
                .await?
                .error_for_status()?
                .json()
                .await?;
        Ok(SearchResults {
            total: response
                .pagination
                .map_or(0, |pagination| pagination.total_count),
            mods: response.data,
        })
    }

    /// Files of a project for this game version and, for mods, this loader. Newest first.
    pub async fn compatible_files(
        &self,
        mod_id: u32,
        game_version: &str,
        loader: Option<Loader>,
    ) -> Result<Vec<File>> {
        let mut params = vec![("gameVersion", game_version.to_owned())];
        if let Some(loader) = loader {
            params.push(("modLoaderType", loader_type(loader).to_string()));
        }
        let response: Response<Vec<File>> = http::send(
            &self.client,
            self.get(&format!("/mods/{mod_id}/files")).query(&params),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        let mut files = response.data;
        // The API lists newest first, but its mirror may not.
        files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
        Ok(files)
    }

    pub async fn mod_info(&self, mod_id: u32) -> Result<Mod> {
        let response: Response<Mod> =
            http::send(&self.client, self.get(&format!("/mods/{mod_id}")))
                .await?
                .error_for_status()?
                .json()
                .await?;
        Ok(response.data)
    }

    /// Top-level category tags for one kind of project.
    pub async fn categories(&self, kind: ContentKind) -> Result<Vec<Category>> {
        let response: Response<Vec<Category>> = http::send(
            &self.client,
            self.get("/categories")
                .query(&[("gameId", MINECRAFT_GAME_ID), ("classId", class_id(kind))]),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response
            .data
            .into_iter()
            // Sub-categories such as add-ons for one mod make the list too long to scan.
            .filter(|category| category.parent_category_id == Some(class_id(kind)))
            .collect())
    }

    /// The project's description as HTML.
    pub async fn description(&self, mod_id: u32) -> Result<String> {
        let response: Response<String> = http::send(
            &self.client,
            self.get(&format!("/mods/{mod_id}/description")),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response.data)
    }

    /// A file's changelog as HTML.
    pub async fn changelog(&self, mod_id: u32, file_id: u32) -> Result<String> {
        let response: Response<String> = http::send(
            &self.client,
            self.get(&format!("/mods/{mod_id}/files/{file_id}/changelog")),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response.data)
    }

    /// Mods by id, in no particular order. Unknown ids are left out.
    pub async fn mods(&self, mod_ids: &[u32]) -> Result<Vec<Mod>> {
        if mod_ids.is_empty() {
            return Ok(Vec::new());
        }
        let response: Response<Vec<Mod>> = http::send(
            &self.client,
            self.request(Method::POST, "/mods")
                .json(&serde_json::json!({ "modIds": mod_ids })),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response.data)
    }

    /// Files by id, in no particular order. Unknown ids are left out.
    pub async fn files(&self, file_ids: &[u32]) -> Result<Vec<File>> {
        if file_ids.is_empty() {
            return Ok(Vec::new());
        }
        let response: Response<Vec<File>> = http::send(
            &self.client,
            self.request(Method::POST, "/mods/files")
                .json(&serde_json::json!({ "fileIds": file_ids })),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response.data)
    }

    /// Looks files up by [`fingerprint`]; unknown fingerprints are left out.
    pub async fn files_by_fingerprint(
        &self,
        fingerprints: &[u32],
    ) -> Result<HashMap<u32, KnownFile>> {
        // A fingerprint always names the same file, so each is asked about once per session.
        static KNOWN: Mutex<Option<HashMap<u32, Option<KnownFile>>>> = Mutex::new(None);
        let lock = || KNOWN.lock().unwrap_or_else(PoisonError::into_inner);

        let unknown: Vec<u32> = {
            let known = lock();
            fingerprints
                .iter()
                .copied()
                .filter(|print| {
                    known
                        .as_ref()
                        .is_none_or(|known| !known.contains_key(print))
                })
                .collect()
        };
        if !unknown.is_empty() {
            let response: Response<FingerprintMatches> = http::send(
                &self.client,
                self.request(Method::POST, &format!("/fingerprints/{MINECRAFT_GAME_ID}"))
                    .json(&serde_json::json!({ "fingerprints": unknown })),
            )
            .await?
            .error_for_status()?
            .json()
            .await?;
            let mut found: HashMap<u32, KnownFile> = response
                .data
                .exact_matches
                .into_iter()
                .map(|found| {
                    let file = found.file;
                    let known = KnownFile {
                        mod_id: file.mod_id,
                        file_id: file.id,
                    };
                    (file.file_fingerprint, known)
                })
                .collect();
            let mut known = lock();
            let known = known.get_or_insert_default();
            for print in unknown {
                known.insert(print, found.remove(&print));
            }
        }

        let known = lock();
        Ok(fingerprints
            .iter()
            .filter_map(|print| Some((*print, (*known.as_ref()?.get(print)?)?)))
            .collect())
    }

    /// The mods plus their required dependencies, newest compatible file of each. Mods in
    /// `installed` and their dependencies are skipped.
    pub async fn resolve_install(
        &self,
        mod_ids: &[u32],
        game_version: &str,
        loader: Option<Loader>,
        installed: impl IntoIterator<Item = u32>,
    ) -> Result<InstallPlan> {
        let mut seen: HashSet<u32> = installed.into_iter().collect();
        let mut queue: Vec<u32> = mod_ids.iter().rev().copied().collect();
        let mut plan = InstallPlan::default();

        while let Some(id) = queue.pop() {
            if !seen.insert(id) {
                continue;
            }
            let files = self.compatible_files(id, game_version, loader).await?;
            let Some(file) = files.into_iter().next() else {
                continue;
            };
            queue.extend(
                file.dependencies
                    .iter()
                    .filter(|dependency| dependency.relation_type == REQUIRED_DEPENDENCY)
                    .map(|dependency| dependency.mod_id),
            );
            match file.remote_file() {
                Some(remote) => plan.files.push((id, remote)),
                None => plan.manual.push(id),
            }
        }
        Ok(plan)
    }
}

/// CurseForge's file fingerprint: 32-bit `MurmurHash2` with seed 1 over the file with all
/// whitespace bytes removed.
pub fn fingerprint(bytes: &[u8]) -> u32 {
    const M: u32 = 0x5bd1_e995;

    let data: Vec<u8> = bytes
        .iter()
        .copied()
        .filter(|byte| !matches!(byte, b'\t' | b'\n' | b'\r' | b' '))
        .collect();
    let mut hash = 1 ^ u32::try_from(data.len()).unwrap_or(u32::MAX);

    let (chunks, rest) = data.as_chunks::<4>();
    for chunk in chunks {
        let mut k = u32::from_le_bytes(*chunk);
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        hash = hash.wrapping_mul(M) ^ k;
    }

    if rest.len() >= 3 {
        hash ^= u32::from(rest[2]) << 16;
    }
    if rest.len() >= 2 {
        hash ^= u32::from(rest[1]) << 8;
    }
    if let Some(&first) = rest.first() {
        hash ^= u32::from(first);
        hash = hash.wrapping_mul(M);
    }

    hash ^= hash >> 13;
    hash = hash.wrapping_mul(M);
    hash ^ (hash >> 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_without_a_download_url_need_the_website() {
        let file: File = serde_json::from_str(
            r#"{
                "id": 1, "modId": 2, "fileName": "a.jar", "downloadUrl": null, "fileLength": 3,
                "hashes": [{ "value": "md5", "algo": 2 }, { "value": "abc", "algo": 1 }],
                "dependencies": [{ "modId": 4, "relationType": 3 }]
            }"#,
        )
        .unwrap();

        assert_eq!(file.sha1(), Some("abc"));
        assert!(file.remote_file().is_none());
    }

    #[test]
    fn fingerprint_ignores_whitespace() {
        assert_eq!(fingerprint(b"hello world\r\n"), fingerprint(b"helloworld"));
        assert_ne!(fingerprint(b"helloworld"), fingerprint(b"helloworle"));
    }
}
