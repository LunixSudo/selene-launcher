use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{Result, http};

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: VersionKind,
    pub url: String,
    pub time: DateTime<Utc>,
    pub release_time: DateTime<Utc>,
    pub sha1: String,
    #[serde(default)]
    pub compliance_level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionKind {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

impl VersionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Snapshot => "snapshot",
            Self::OldBeta => "old_beta",
            Self::OldAlpha => "old_alpha",
        }
    }
}

impl VersionManifest {
    pub async fn fetch(client: &Client) -> Result<Self> {
        http::get_json(client, VERSION_MANIFEST_URL).await
    }

    pub fn get(&self, id: &str) -> Option<&VersionEntry> {
        self.versions.iter().find(|version| version.id == id)
    }

    /// Versions of the given kind, newest first, as Mojang orders them.
    pub fn of_kind(&self, kind: VersionKind) -> impl Iterator<Item = &VersionEntry> {
        self.versions
            .iter()
            .filter(move |version| version.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "latest": { "release": "1.21.10", "snapshot": "25w43a" },
        "versions": [
            {
                "id": "25w43a",
                "type": "snapshot",
                "url": "https://piston-meta.mojang.com/v1/packages/aa/25w43a.json",
                "time": "2025-10-21T12:00:00+00:00",
                "releaseTime": "2025-10-21T11:50:00+00:00",
                "sha1": "aa",
                "complianceLevel": 1
            },
            {
                "id": "1.21.10",
                "type": "release",
                "url": "https://piston-meta.mojang.com/v1/packages/bb/1.21.10.json",
                "time": "2025-10-07T09:00:00+00:00",
                "releaseTime": "2025-10-07T08:00:00+00:00",
                "sha1": "bb",
                "complianceLevel": 1
            },
            {
                "id": "b1.7.3",
                "type": "old_beta",
                "url": "https://piston-meta.mojang.com/v1/packages/cc/b1.7.3.json",
                "time": "2010-07-08T22:00:00+00:00",
                "releaseTime": "2011-07-07T22:00:00+00:00",
                "sha1": "cc"
            }
        ]
    }"#;

    #[test]
    fn parses_manifest() {
        let manifest: VersionManifest = serde_json::from_str(SAMPLE).unwrap();

        assert_eq!(manifest.latest.release, "1.21.10");
        assert_eq!(manifest.versions.len(), 3);
        assert_eq!(manifest.versions[2].kind, VersionKind::OldBeta);
        assert_eq!(manifest.versions[2].compliance_level, 0);
    }

    #[test]
    fn filters_by_kind() {
        let manifest: VersionManifest = serde_json::from_str(SAMPLE).unwrap();
        let releases: Vec<_> = manifest
            .of_kind(VersionKind::Release)
            .map(|version| version.id.as_str())
            .collect();

        assert_eq!(releases, ["1.21.10"]);
        assert_eq!(manifest.get("b1.7.3").unwrap().sha1, "cc");
        assert!(manifest.get("1.0.0").is_none());
    }
}
