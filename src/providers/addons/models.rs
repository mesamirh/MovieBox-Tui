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

/// `videoSize` arrives as an integer from most addons, but some send a float or
/// a numeric string. Anything unreadable becomes `None` instead of failing.
fn de_opt_u64_lenient<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <Option<serde_json::Value> as serde::Deserialize>::deserialize(deserializer)?;
    let non_negative = |f: f64| (f.is_finite() && f >= 0.0).then_some(f as u64);
    Ok(match value {
        Some(serde_json::Value::Number(n)) => {
            n.as_u64().or_else(|| n.as_f64().and_then(non_negative))
        }
        Some(serde_json::Value::String(s)) => s
            .trim()
            .parse::<u64>()
            .ok()
            .or_else(|| s.trim().parse::<f64>().ok().and_then(non_negative)),
        _ => None,
    })
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamBehaviorHints {
    #[serde(rename = "notWebReady", default)]
    pub not_web_ready: bool,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(rename = "videoSize", default, deserialize_with = "de_opt_u64_lenient")]
    pub video_size: Option<u64>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamItem {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "behaviorHints")]
    pub behavior_hints: Option<StreamBehaviorHints>,
    /// Torrent info-hash (40 hex chars) for P2P streams.
    #[serde(rename = "infoHash", default)]
    pub info_hash: Option<String>,
    /// Zero-based index of the video file inside the torrent.
    #[serde(
        rename = "fileIdx",
        default,
        deserialize_with = "de_opt_usize_or_string"
    )]
    pub file_idx: Option<usize>,
    /// Tracker (`tracker:udp://...`) and DHT (`dht:<hash>`) hints for the swarm.
    #[serde(default, deserialize_with = "de_string_or_vec")]
    pub sources: Vec<String>,
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
    #[serde(default)]
    pub catalogs: Vec<CatalogDefinition>,
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
            catalogs: Vec::new(),
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
            catalogs: manifest.catalogs.clone(),
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

const MAX_BROWSE_CATALOGS: usize = 32;

fn humanize_catalog_token(raw: &str) -> String {
    match raw.trim() {
        "top" | "popular" | "Popular" => "Top".to_string(),
        "imdbRating" | "Featured" | "featured" => "Top Rated".to_string(),
        "year" | "New" => "New".to_string(),
        other if !other.contains(' ') && (other.contains('-') || other.contains('_')) => other
            .split(['-', '_'])
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        other => other.to_string(),
    }
}

fn format_catalog_label(cat: &CatalogDefinition, addon_name: &str, multi: bool) -> String {
    let type_label = match cat.r#type.to_ascii_lowercase().as_str() {
        "movie" | "movies" => "Movies".to_string(),
        "series" | "tv" | "show" | "shows" => "Series".to_string(),
        "anime" => "Anime".to_string(),
        "channel" | "channels" => "Channels".to_string(),
        other => humanize_catalog_token(other),
    };
    let raw_name = cat
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(cat.id.as_str());
    let clean_name = humanize_catalog_token(raw_name);
    let lower_name = clean_name.to_ascii_lowercase();
    let lower_type = type_label.to_ascii_lowercase();
    let base = if clean_name.is_empty() || lower_name == lower_type {
        format!("Top {type_label}")
    } else if lower_name.contains(&lower_type)
        || (lower_type == "movies" && lower_name.contains("movie"))
        || (lower_type == "series"
            && (lower_name.contains("series") || lower_name.contains("show")))
    {
        clean_name
    } else {
        format!("{clean_name} {type_label}")
    };
    if multi
        && !base
            .to_ascii_lowercase()
            .contains(&addon_name.to_ascii_lowercase())
    {
        format!("{base} ({addon_name})")
    } else {
        base
    }
}

fn push_unique_target(targets: &mut Vec<AddonCatalogTarget>, target: AddonCatalogTarget) {
    if targets.len() < MAX_BROWSE_CATALOGS
        && !targets.iter().any(|existing| {
            existing.manifest_url == target.manifest_url
                && existing.r#type.eq_ignore_ascii_case(&target.r#type)
                && existing.catalog_id == target.catalog_id
        })
    {
        targets.push(target);
    }
}

pub fn curated_catalog_presets(addons: &[InstalledAddon]) -> Vec<AddonCatalogTarget> {
    let mut targets: Vec<AddonCatalogTarget> = Vec::new();
    let enabled_catalogs: Vec<&InstalledAddon> = addons
        .iter()
        .filter(|a| a.enabled && (a.provides_catalog || !a.catalogs.is_empty()))
        .collect();

    let multi = enabled_catalogs.len() > 1;

    for addon in &enabled_catalogs {
        let suffix = if multi {
            format!(" ({})", addon.name)
        } else {
            String::new()
        };

        let browsable: Vec<&CatalogDefinition> = addon
            .catalogs
            .iter()
            .filter(|c| {
                !c.r#type.trim().is_empty()
                    && !c.id.trim().is_empty()
                    && !c.extra.iter().any(|e| e.is_required)
            })
            .collect();

        if !browsable.is_empty() {
            for cat in browsable {
                push_unique_target(
                    &mut targets,
                    AddonCatalogTarget {
                        label: format_catalog_label(cat, &addon.name, multi),
                        addon_name: addon.name.clone(),
                        manifest_url: addon.manifest_url.clone(),
                        r#type: cat.r#type.clone(),
                        catalog_id: cat.id.clone(),
                    },
                );
            }
            continue;
        }

        let has_movies =
            addon.types.is_empty() || addon.types.iter().any(|t| t.eq_ignore_ascii_case("movie"));
        let has_series =
            addon.types.is_empty() || addon.types.iter().any(|t| t.eq_ignore_ascii_case("series"));

        if has_movies {
            push_unique_target(
                &mut targets,
                AddonCatalogTarget {
                    label: format!("Top Movies{suffix}"),
                    addon_name: addon.name.clone(),
                    manifest_url: addon.manifest_url.clone(),
                    r#type: "movie".to_string(),
                    catalog_id: "top".to_string(),
                },
            );
        }
        if has_series {
            push_unique_target(
                &mut targets,
                AddonCatalogTarget {
                    label: format!("Top Series{suffix}"),
                    addon_name: addon.name.clone(),
                    manifest_url: addon.manifest_url.clone(),
                    r#type: "series".to_string(),
                    catalog_id: "top".to_string(),
                },
            );
        }
        if has_movies && addon.is_core() {
            push_unique_target(
                &mut targets,
                AddonCatalogTarget {
                    label: format!("Top Rated Movies{suffix}"),
                    addon_name: addon.name.clone(),
                    manifest_url: addon.manifest_url.clone(),
                    r#type: "movie".to_string(),
                    catalog_id: "imdbRating".to_string(),
                },
            );
        }
        if has_series && addon.is_core() {
            push_unique_target(
                &mut targets,
                AddonCatalogTarget {
                    label: format!("Top Rated Series{suffix}"),
                    addon_name: addon.name.clone(),
                    manifest_url: addon.manifest_url.clone(),
                    r#type: "series".to_string(),
                    catalog_id: "imdbRating".to_string(),
                },
            );
        }
        if targets.len() >= MAX_BROWSE_CATALOGS {
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

    targets.truncate(MAX_BROWSE_CATALOGS);
    targets
}

/// Finds a catalog target matching a `/browse <filter>` query by label, addon
/// name, catalog ID, or media type.
pub fn find_catalog_preset(addons: &[InstalledAddon], filter: &str) -> Option<AddonCatalogTarget> {
    let needle = filter.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    let presets = curated_catalog_presets(addons);
    presets
        .iter()
        .find(|t| t.label.to_ascii_lowercase() == needle)
        .or_else(|| {
            presets
                .iter()
                .find(|t| t.label.to_ascii_lowercase().starts_with(&needle))
        })
        .or_else(|| {
            presets.iter().find(|t| {
                t.label.to_ascii_lowercase().contains(&needle)
                    || t.addon_name.to_ascii_lowercase().contains(&needle)
                    || t.catalog_id.to_ascii_lowercase().contains(&needle)
                    || t.r#type.to_ascii_lowercase().contains(&needle)
            })
        })
        .cloned()
}
