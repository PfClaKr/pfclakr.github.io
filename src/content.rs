//! Loads markdown content from `content/`.
//!
//! ```text
//! content/
//!   home/<lang>.md                  → bio on the home page (business card)
//!   pages/<key>/<lang>.md           → /<lang-prefix>/<key>/
//!   posts/<key>/<lang>.md (+assets) → /<lang-prefix>/posts/<slug>/
//! ```
//! Files sharing the same `<key>` directory are translations of each other.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::config::Config;
use crate::markdown::{self, Rendered};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontMatter {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<toml::value::Datetime>,
    pub updated: Option<toml::value::Datetime>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub draft: bool,
    /// Social preview image (absolute path or URL).
    pub image: Option<String>,
    /// Override the URL slug for this language.
    pub slug: Option<String>,
    #[serde(default = "yes")]
    pub toc: bool,
    #[serde(default = "yes")]
    pub comments: bool,
    /// Pages only: show in the navigation bar.
    #[serde(default)]
    pub in_menu: bool,
    /// Pages only: ordering in the navigation bar.
    #[serde(default)]
    pub weight: i32,
}

fn yes() -> bool {
    true
}

#[derive(Debug)]
pub struct Doc {
    /// Directory name shared by translations.
    pub key: String,
    pub lang: String,
    pub slug: String,
    pub meta: FrontMatter,
    pub date: Option<NaiveDate>,
    pub updated: Option<NaiveDate>,
    pub rendered: Rendered,
    pub source: PathBuf,
}

impl Doc {
    pub fn description(&self) -> &str {
        self.meta
            .description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or(&self.rendered.summary)
    }

    pub fn reading_minutes(&self, wpm: usize) -> usize {
        (self.rendered.word_count / wpm.max(1)).max(1)
    }
}

/// A post or page with all its translations, keyed by language.
#[derive(Debug)]
pub struct Group {
    pub key: String,
    pub docs: BTreeMap<String, Doc>,
    /// Non-markdown files in the directory (images…).
    pub assets: Vec<PathBuf>,
}

impl Group {
    /// Best available translation for `lang`, following the fallback order.
    pub fn best<'a>(&'a self, lang: &str, cfg: &Config) -> Option<&'a Doc> {
        self.docs.get(lang).or_else(|| {
            cfg.fallback_order()
                .into_iter()
                .find_map(|l| self.docs.get(l))
        })
    }
}

pub struct Content {
    pub home: BTreeMap<String, Rendered>,
    pub pages: Vec<Group>,
    /// Sorted newest first.
    pub posts: Vec<Group>,
}

/// The slug used for a post's asset folder: directory name without a leading date.
pub fn base_slug(key: &str) -> String {
    let b = key.as_bytes();
    let has_date = b.len() > 11
        && b[..10].iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
        && b[10] == b'-';
    if has_date {
        key[11..].to_string()
    } else {
        key.to_string()
    }
}

fn date_prefix(key: &str) -> Option<NaiveDate> {
    key.get(..10)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

fn to_date(dt: &Option<toml::value::Datetime>, src: &Path) -> Result<Option<NaiveDate>> {
    let Some(dt) = dt else { return Ok(None) };
    let Some(d) = dt.date else {
        bail!("{}: date without a day component", src.display())
    };
    NaiveDate::from_ymd_opt(d.year as i32, d.month as u32, d.day as u32)
        .map(Some)
        .with_context(|| format!("{}: invalid date", src.display()))
}

/// Splits `+++ toml +++ body` front matter.
fn split_front_matter(raw: &str, src: &Path) -> Result<(FrontMatter, String)> {
    let raw = raw.trim_start_matches('\u{feff}');
    let Some(rest) = raw.strip_prefix("+++") else {
        bail!("{}: missing `+++` TOML front matter", src.display());
    };
    let Some(end) = rest.find("\n+++") else {
        bail!(
            "{}: unterminated front matter (closing `+++` not found)",
            src.display()
        );
    };
    let fm: FrontMatter = toml::from_str(&rest[..end])
        .with_context(|| format!("{}: invalid front matter", src.display()))?;
    let body = rest[end + 4..].trim_start_matches(['\r', '\n']).to_string();
    Ok((fm, body))
}

fn load_doc(path: &Path, key: &str, lang: &str, asset_base: Option<&str>) -> Result<Doc> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let (meta, body) = split_front_matter(&raw, path)?;
    if meta.title.trim().is_empty() {
        bail!("{}: `title` must not be empty", path.display());
    }
    let date = to_date(&meta.date, path)?.or_else(|| date_prefix(key));
    let updated = to_date(&meta.updated, path)?;
    let slug = meta.slug.clone().unwrap_or_else(|| base_slug(key));
    Ok(Doc {
        key: key.to_string(),
        lang: lang.to_string(),
        slug,
        date,
        updated,
        rendered: markdown::render(&body, asset_base),
        meta,
        source: path.to_path_buf(),
    })
}

fn load_groups(
    dir: &Path,
    cfg: &Config,
    drafts: bool,
    with_assets: Option<&str>,
) -> Result<Vec<Group>> {
    let mut groups = Vec::new();
    if !dir.exists() {
        return Ok(groups);
    }
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let key = entry.file_name().to_string_lossy().to_string();
        let asset_base = with_assets.map(|section| format!("/{section}/{}/", base_slug(&key)));
        let mut docs = BTreeMap::new();
        let mut assets = Vec::new();
        for f in fs::read_dir(entry.path())? {
            let path = f?.path();
            if !path.is_file() {
                continue;
            }
            let is_md = path.extension().is_some_and(|e| e == "md");
            if !is_md {
                assets.push(path);
                continue;
            }
            let lang = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !cfg.languages.contains(&lang) {
                bail!(
                    "{}: `{lang}` is not a configured language ({:?})",
                    path.display(),
                    cfg.languages
                );
            }
            let doc = load_doc(&path, &key, &lang, asset_base.as_deref())?;
            if doc.meta.draft && !drafts {
                continue;
            }
            docs.insert(lang, doc);
        }
        if !docs.is_empty() {
            groups.push(Group { key, docs, assets });
        }
    }
    Ok(groups)
}

pub fn load(root: &Path, cfg: &Config, drafts: bool) -> Result<Content> {
    let content = root.join("content");

    let mut home = BTreeMap::new();
    for lang in &cfg.languages {
        let p = content.join("home").join(format!("{lang}.md"));
        if p.exists() {
            let raw = fs::read_to_string(&p)?;
            home.insert(lang.clone(), markdown::render(&raw, None));
        }
    }

    let mut pages = load_groups(&content.join("pages"), cfg, drafts, None)?;
    pages.sort_by(|a, b| {
        let w = |g: &Group| g.docs.values().next().map(|d| d.meta.weight).unwrap_or(0);
        w(a).cmp(&w(b)).then_with(|| a.key.cmp(&b.key))
    });

    let mut posts = load_groups(&content.join("posts"), cfg, drafts, Some("posts"))?;
    for g in &posts {
        for d in g.docs.values() {
            if d.date.is_none() {
                bail!(
                    "{}: posts need a `date` (front matter or YYYY-MM-DD- directory prefix)",
                    d.source.display()
                );
            }
        }
    }
    let newest = |g: &Group| g.docs.values().filter_map(|d| d.date).max();
    posts.sort_by(|a, b| newest(b).cmp(&newest(a)).then_with(|| a.key.cmp(&b.key)));

    Ok(Content { home, pages, posts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_date_prefix() {
        assert_eq!(base_slug("2026-11-02-hello-world"), "hello-world");
        assert_eq!(base_slug("hello-world"), "hello-world");
        assert_eq!(base_slug("2026-11-02"), "2026-11-02");
    }

    #[test]
    fn parses_front_matter() {
        let (fm, body) = split_front_matter(
            "+++\ntitle = \"Hi\"\ntags = [\"rust\"]\n+++\n\n# Body",
            Path::new("x.md"),
        )
        .unwrap();
        assert_eq!(fm.title, "Hi");
        assert_eq!(fm.tags, ["rust"]);
        assert_eq!(body, "# Body");
    }
}
