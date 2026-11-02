use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub base_url: String,
    pub default_language: String,
    pub languages: Vec<String>,
    #[serde(default = "yes")]
    pub show_untranslated: bool,
    #[serde(default = "default_wpm")]
    pub words_per_minute: usize,
    pub author: Author,
    #[serde(default)]
    pub seo: Seo,
    #[serde(default)]
    pub comments: Comments,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub linkedin: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub twitter: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Seo {
    #[serde(default)]
    pub og_image: String,
    #[serde(default)]
    pub google_site_verification: String,
    #[serde(default)]
    pub naver_site_verification: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Comments {
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub repo_id: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub category_id: String,
}

impl Comments {
    pub fn enabled(&self) -> bool {
        !self.repo.is_empty() && !self.repo_id.is_empty() && !self.category_id.is_empty()
    }
}

fn yes() -> bool {
    true
}
fn default_wpm() -> usize {
    220
}

/// UI strings for one language (`i18n/<lang>.toml`).
/// Typed fields are the ones the builder itself needs; the whole table is
/// also handed to templates as `t`.
#[derive(Debug, Clone)]
pub struct Strings {
    pub language_name: String,
    pub locale: String,
    pub site_title: String,
    pub site_description: String,
    pub date_format: String,
    pub months: Vec<String>,
    pub table: toml::Table,
}

impl Strings {
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.table.get(key).and_then(|v| v.as_str()).unwrap_or(key)
    }

    pub fn format_date(&self, d: chrono::NaiveDate) -> String {
        use chrono::Datelike;
        let month = self
            .months
            .get(d.month0() as usize)
            .cloned()
            .unwrap_or_else(|| d.month().to_string());
        self.date_format
            .replace("{year}", &d.year().to_string())
            .replace("{month}", &month)
            .replace("{day}", &d.day().to_string())
    }
}

impl Config {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("config.toml");
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let mut cfg: Config =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        cfg.base_url = cfg.base_url.trim_end_matches('/').to_string();
        if !cfg.languages.contains(&cfg.default_language) {
            bail!(
                "default_language `{}` must be listed in `languages`",
                cfg.default_language
            );
        }
        Ok(cfg)
    }

    /// URL prefix for a language: "" for the default one, "/fr" otherwise.
    pub fn prefix(&self, lang: &str) -> String {
        if lang == self.default_language {
            String::new()
        } else {
            format!("/{lang}")
        }
    }

    /// Languages ordered for fallback: default first, then the configured order.
    pub fn fallback_order(&self) -> Vec<&str> {
        let mut v = vec![self.default_language.as_str()];
        v.extend(
            self.languages
                .iter()
                .map(String::as_str)
                .filter(|l| *l != self.default_language),
        );
        v
    }
}

pub fn load_strings(root: &Path, cfg: &Config) -> Result<BTreeMap<String, Strings>> {
    let mut map = BTreeMap::new();
    for lang in &cfg.languages {
        let path = root.join("i18n").join(format!("{lang}.toml"));
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let table: toml::Table =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        let s = |k: &str| {
            table
                .get(k)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let months = table
            .get("months")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|m| m.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let strings = Strings {
            language_name: s("language_name"),
            locale: s("locale"),
            site_title: s("site_title"),
            site_description: s("site_description"),
            date_format: if s("date_format").is_empty() {
                "{year}-{month}-{day}".into()
            } else {
                s("date_format")
            },
            months,
            table,
        };
        map.insert(lang.clone(), strings);
    }
    Ok(map)
}
