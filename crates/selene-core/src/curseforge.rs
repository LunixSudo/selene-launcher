use std::collections::HashSet;

use reqwest::{Client, RequestBuilder};
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
        let request = self.client.get(format!("{API_URL}{path}"));
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
}
