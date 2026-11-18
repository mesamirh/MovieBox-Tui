use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AddonResource {
    Simple(String),
    Detailed {
        name: String,
        types: Option<Vec<String>>,
        #[serde(rename = "idPrefixes")]
        id_prefixes: Option<Vec<String>>,
    },
}

impl AddonResource {
    pub fn name(&self) -> &str {
        match self {
            Self::Simple(name) => name.as_str(),
            Self::Detailed { name, .. } => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogExtra {
    pub name: String,
    #[serde(rename = "isRequired", default)]
    pub is_required: bool,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogDefinition {
    #[serde(rename = "type")]
    pub r#type: String,
    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub extra: Vec<CatalogExtra>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddonManifest {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub resources: Vec<AddonResource>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub catalogs: Vec<CatalogDefinition>,
    #[serde(rename = "idPrefixes", default)]
    pub id_prefixes: Vec<String>,
    pub logo: Option<String>,
    pub background: Option<String>,
}

impl AddonManifest {
    pub fn provides_resource(&self, res_name: &str) -> bool {
        self.resources
            .iter()
            .any(|r| r.name().eq_ignore_ascii_case(res_name))
    }

    pub fn provides_stream(&self) -> bool {
        self.provides_resource("stream")
    }

    pub fn provides_meta(&self) -> bool {
        self.provides_resource("meta")
    }

    pub fn provides_catalog(&self) -> bool {
        self.provides_resource("catalog") || !self.catalogs.is_empty()
    }
}

fn de_opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrNumberVisitor;
    impl<'de> serde::de::Visitor<'de> for StringOrNumberVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, number, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            if v.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(v.trim().to_string()))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
            if v.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(v.trim().to_string()))
            }
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}

fn de_opt_usize_or_string<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct UsizeOrStringVisitor;
    impl<'de> serde::de::Visitor<'de> for UsizeOrStringVisitor {
        type Value = Option<usize>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a usize, string, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as usize))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            if v >= 0 {
                Ok(Some(v as usize))
            } else {
                Ok(None)
            }
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
            if v >= 0.0 {
                Ok(Some(v as usize))
            } else {
                Ok(None)
            }
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.trim().parse::<usize>().ok())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
            Ok(v.trim().parse::<usize>().ok())
        }
    }

    deserializer.deserialize_any(UsizeOrStringVisitor)
}

fn de_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrVecVisitor;

    impl<'de> serde::de::Visitor<'de> for StringOrVecVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, sequence of strings, or sequence of objects")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut list = Vec::new();
            while let Some(val) = seq.next_element::<serde_json::Value>()? {
                if let Some(s) = val.as_str() {
                    let s_trim = s.trim();
                    if !s_trim.is_empty() {
                        list.push(s_trim.to_string());
                    }
                } else if let Some(name) = val.get("name").and_then(|n| n.as_str()) {
                    let n_trim = name.trim();
                    if !n_trim.is_empty() {
                        list.push(n_trim.to_string());
                    }
                }
            }
            Ok(list)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(vec![v.to_string()])
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(vec![v.to_string()])
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(vec![v.to_string()])
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Vec::new())
        }
    }

    deserializer.deserialize_any(StringOrVecVisitor)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaItem {
    pub id: String,
    #[serde(rename = "type", default)]
    pub r#type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub poster: Option<String>,
    pub cover: Option<String>,
    pub description: Option<String>,
    pub overview: Option<String>,
    pub synopsis: Option<String>,
    #[serde(
        rename = "releaseInfo",
        default,
        deserialize_with = "de_opt_string_or_number"
    )]
    pub release_info: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub year: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub released: Option<String>,
    #[serde(
        rename = "imdbRating",
        default,
        deserialize_with = "de_opt_string_or_number"
    )]
    pub imdb_rating: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub rating: Option<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub genres: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub genre: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogResponse {
    #[serde(default)]
    pub metas: Vec<MetaItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaVideo {
    #[serde(default)]
    pub id: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    #[serde(default, deserialize_with = "de_opt_usize_or_string")]
    pub season: Option<usize>,
    #[serde(default, deserialize_with = "de_opt_usize_or_string")]
    pub episode: Option<usize>,
    #[serde(default, deserialize_with = "de_opt_usize_or_string")]
    pub number: Option<usize>,
    pub released: Option<String>,
    pub thumbnail: Option<String>,
    pub overview: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaDetail {
    pub id: String,
    #[serde(rename = "type", default)]
    pub r#type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub poster: Option<String>,
    pub cover: Option<String>,
    pub background: Option<String>,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub overview: Option<String>,
    pub synopsis: Option<String>,
    #[serde(
        rename = "releaseInfo",
        default,
        deserialize_with = "de_opt_string_or_number"
    )]
    pub release_info: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub year: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub released: Option<String>,
    #[serde(
        rename = "imdbRating",
        default,
        deserialize_with = "de_opt_string_or_number"
    )]
    pub imdb_rating: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub rating: Option<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub genres: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub genre: Vec<String>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub runtime: Option<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub cast: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub stars: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub director: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub directors: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub writer: Vec<String>,
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub writers: Vec<String>,
    #[serde(default)]
    pub videos: Vec<MetaVideo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamBehaviorHints {
    #[serde(rename = "notWebReady", default)]
    pub not_web_ready: bool,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(rename = "proxyHeaders", default)]
    pub proxy_headers: Option<serde_json::Value>,
    #[serde(rename = "videoSize")]
    pub video_size: Option<u64>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamItem {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "behaviorHints")]
    pub behavior_hints: Option<StreamBehaviorHints>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledAddon {
    pub manifest_url: String,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub provides_catalog: bool,
    #[serde(default)]
    pub provides_meta: bool,
    #[serde(default)]
    pub provides_stream: bool,
    #[serde(default)]
    pub id_prefixes: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
}

impl InstalledAddon {
    pub const CINEMETA_MANIFEST: &'static str = "https://v3-cinemeta.strem.io/manifest.json";

    pub fn cinemeta_default() -> Self {
        Self {
            manifest_url: Self::CINEMETA_MANIFEST.to_string(),
            name: "Cinemeta".to_string(),
            version: Some("3.0.14".to_string()),
            description: Some("Official Catalog and Metadata".to_string()),
            enabled: true,
            provides_catalog: true,
            provides_meta: true,
            provides_stream: false,
            id_prefixes: vec!["tt".to_string()],
            types: vec!["movie".to_string(), "series".to_string()],
        }
    }

    pub fn is_core(&self) -> bool {
        self.name.eq_ignore_ascii_case("cinemeta")
            || self.manifest_url.to_lowercase().contains("cinemeta")
    }

    pub fn from_manifest(manifest_url: String, manifest: &AddonManifest) -> Self {
        Self {
            manifest_url,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            enabled: true,
            provides_catalog: manifest.provides_catalog(),
            provides_meta: manifest.provides_meta(),
            provides_stream: manifest.provides_stream(),
            id_prefixes: manifest.id_prefixes.clone(),
            types: manifest.types.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonCatalogTarget {
    pub label: String,
    pub addon_name: String,
    pub manifest_url: String,
    pub r#type: String,
    pub catalog_id: String,
}

pub fn curated_catalog_presets(addons: &[InstalledAddon]) -> Vec<AddonCatalogTarget> {
    let mut targets = Vec::new();
    let enabled_catalogs: Vec<&InstalledAddon> = addons
        .iter()
        .filter(|a| a.enabled && a.provides_catalog)
        .collect();

    let multi = enabled_catalogs.len() > 1;

    for addon in &enabled_catalogs {
        let suffix = if multi {
            format!(" ({})", addon.name)
        } else {
            String::new()
        };

        let has_movies =
            addon.types.is_empty() || addon.types.iter().any(|t| t.eq_ignore_ascii_case("movie"));
        let has_series =
            addon.types.is_empty() || addon.types.iter().any(|t| t.eq_ignore_ascii_case("series"));

        if has_movies {
            targets.push(AddonCatalogTarget {
                label: format!("Top Movies{suffix}"),
                addon_name: addon.name.clone(),
                manifest_url: addon.manifest_url.clone(),
                r#type: "movie".to_string(),
                catalog_id: "top".to_string(),
            });
        }
        if has_series {
            targets.push(AddonCatalogTarget {
                label: format!("Top Series{suffix}"),
                addon_name: addon.name.clone(),
                manifest_url: addon.manifest_url.clone(),
                r#type: "series".to_string(),
                catalog_id: "top".to_string(),
            });
        }
        if has_movies && addon.is_core() {
            targets.push(AddonCatalogTarget {
                label: format!("Top Rated Movies{suffix}"),
                addon_name: addon.name.clone(),
                manifest_url: addon.manifest_url.clone(),
                r#type: "movie".to_string(),
                catalog_id: "imdbRating".to_string(),
            });
        }
        if has_series && addon.is_core() {
            targets.push(AddonCatalogTarget {
                label: format!("Top Rated Series{suffix}"),
                addon_name: addon.name.clone(),
                manifest_url: addon.manifest_url.clone(),
                r#type: "series".to_string(),
                catalog_id: "imdbRating".to_string(),
            });
        }
        if targets.len() >= 6 {
            break;
        }
    }

    if targets.is_empty() {
        let cinemeta = InstalledAddon::cinemeta_default();
        targets.push(AddonCatalogTarget {
            label: "Top Movies".to_string(),
            addon_name: cinemeta.name.clone(),
            manifest_url: cinemeta.manifest_url.clone(),
            r#type: "movie".to_string(),
            catalog_id: "top".to_string(),
        });
        targets.push(AddonCatalogTarget {
            label: "Top Series".to_string(),
            addon_name: cinemeta.name.clone(),
            manifest_url: cinemeta.manifest_url.clone(),
            r#type: "series".to_string(),
            catalog_id: "top".to_string(),
        });
        targets.push(AddonCatalogTarget {
            label: "Top Rated Movies".to_string(),
            addon_name: cinemeta.name.clone(),
            manifest_url: cinemeta.manifest_url.clone(),
            r#type: "movie".to_string(),
            catalog_id: "imdbRating".to_string(),
        });
        targets.push(AddonCatalogTarget {
            label: "Top Rated Series".to_string(),
            addon_name: cinemeta.name.clone(),
            manifest_url: cinemeta.manifest_url.clone(),
            r#type: "series".to_string(),
            catalog_id: "imdbRating".to_string(),
        });
    }

    targets.truncate(6);
    targets
}
