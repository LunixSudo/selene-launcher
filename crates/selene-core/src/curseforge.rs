use std::collections::{HashMap, HashSet};

use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::loader::Loader;
use crate::mods::RemoteFile;

const API_URL: &str = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID: u32 = 432;
const MODS_CLASS_ID: u32 = 6;
const SORT_BY_POPULARITY: u32 = 2;
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
pub struct File {
    pub id: u32,
    pub mod_id: u32,
    pub file_name: String,
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
pub struct SearchResults {
    pub mods: Vec<Mod>,
    pub total: u32,
}

/// What resolving an install produced: files to download, and mods whose authors only allow
/// downloads from the CurseForge website.
#[derive(Debug, Clone, Default)]
pub struct InstallPlan {
    pub files: Vec<RemoteFile>,
    pub manual: Vec<u32>,
}

fn loader_type(loader: Loader) -> u32 {
    match loader {
        Loader::Fabric => 4,
        Loader::Quilt => 5,
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

    pub async fn search(
        &self,
        text: &str,
        game_version: &str,
        loader: Loader,
        offset: u32,
        limit: u32,
    ) -> Result<SearchResults> {
        let response: Response<Vec<Mod>> = self
            .get("/mods/search")
            .query(&[
                ("gameId", MINECRAFT_GAME_ID.to_string()),
                ("classId", MODS_CLASS_ID.to_string()),
                ("gameVersion", game_version.to_owned()),
                ("modLoaderType", loader_type(loader).to_string()),
                ("searchFilter", text.to_owned()),
                ("sortField", SORT_BY_POPULARITY.to_string()),
                ("sortOrder", "desc".to_owned()),
                ("index", offset.to_string()),
                ("pageSize", limit.to_string()),
            ])
            .send()
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

    /// Files of a mod for this game version and loader, newest first.
    pub async fn compatible_files(
        &self,
        mod_id: u32,
        game_version: &str,
        loader: Loader,
    ) -> Result<Vec<File>> {
        let response: Response<Vec<File>> = self
            .get(&format!("/mods/{mod_id}/files"))
            .query(&[
                ("gameVersion", game_version.to_owned()),
                ("modLoaderType", loader_type(loader).to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response.data)
    }

    pub async fn mod_info(&self, mod_id: u32) -> Result<Mod> {
        let response: Response<Mod> = self
            .get(&format!("/mods/{mod_id}"))
            .send()
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
        if fingerprints.is_empty() {
            return Ok(HashMap::new());
        }
        let response: Response<FingerprintMatches> = self
            .request(Method::POST, &format!("/fingerprints/{MINECRAFT_GAME_ID}"))
            .json(&serde_json::json!({ "fingerprints": fingerprints }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response
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
            .collect())
    }

    /// The mod plus its required dependencies, newest compatible file of each.
    pub async fn resolve_install(
        &self,
        mod_id: u32,
        game_version: &str,
        loader: Loader,
    ) -> Result<InstallPlan> {
        let mut seen = HashSet::new();
        let mut queue = vec![mod_id];
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
                Some(remote) => plan.files.push(remote),
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
