//! Loads markdown content from `content/`.
//!
//! ```text
//! content/
//!   home/<lang>.md                  → bio on the home page (business card)
//!   pages/<key>/<lang>.md           → /<lang-prefix>/<key>/
//!   posts/[<dir>/…]<key>/<lang>.md   → /<lang-prefix>/posts/[<dir>/…]<slug>/
//!   posts/<dir>/_dir.toml             → optional localized name of a folder
//! ```
//! Files sharing the same `<key>` directory are translations of each other.
//! Posts can be grouped in folders of any depth (`posts/unix/bash/…`); the
//! folder path becomes part of the URL and gets its own listing page. A
//! directory that contains `<lang>.md` files is a post, otherwise a folder.

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

/// `_dir.toml`: how a post folder is presented.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirMeta {
    /// Localized display name, e.g. `title = { en = "Architecture", ko = "아키텍처" }`.
    #[serde(default)]
    pub title: BTreeMap<String, String>,
    #[serde(default)]
    pub description: BTreeMap<String, String>,
    /// Ordering among sibling folders (then by name).
    #[serde(default)]
    pub weight: i32,
}

#[derive(Debug)]
pub struct Doc {
    /// Directory name shared by translations.
    pub key: String,
    /// Folder path under `content/posts` (`""`, `"42"`, `"unix/bash"`).
    pub dir: String,
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
    /// Folder path under `content/posts` (`""` for pages and top-level posts).
    pub dir: String,
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
    /// Every folder that (transitively) contains a post, keyed by path.
    pub dirs: BTreeMap<String, DirMeta>,
}

impl Group {
    /// Path used for URLs and asset folders: `<dir>/<slug>` or `<slug>`.
    pub fn path_of(dir: &str, slug: &str) -> String {
        if dir.is_empty() {
            slug.to_string()
        } else {
            format!("{dir}/{slug}")
        }
    }
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

fn load_doc(
    path: &Path,
    key: &str,
    dir: &str,
    lang: &str,
    asset_base: Option<&str>,
) -> Result<Doc> {
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
        dir: dir.to_string(),
        lang: lang.to_string(),
        slug,
        date,
        updated,
        rendered: markdown::render(&body, asset_base),
        meta,
        source: path.to_path_buf(),
    })
}

fn is_markdown(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|e| e == "md")
}

/// Loads one post/page directory (all its translations and assets).
fn load_group(
    path: &Path,
    key: &str,
    dir: &str,
    cfg: &Config,
    drafts: bool,
    asset_base: Option<&str>,
) -> Result<Option<Group>> {
    let mut docs = BTreeMap::new();
    let mut assets = Vec::new();
    let mut files: Vec<PathBuf> = fs::read_dir(path)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    for path in files {
        if !is_markdown(&path) {
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
        let doc = load_doc(&path, key, dir, &lang, asset_base)?;
        if doc.meta.draft && !drafts {
            continue;
        }
        docs.insert(lang, doc);
    }
    Ok((!docs.is_empty()).then(|| Group {
        key: key.to_string(),
        dir: dir.to_string(),
        docs,
        assets,
    }))
}

fn sorted_subdirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn file_name(p: &Path) -> String {
    p.file_name().unwrap_or_default().to_string_lossy().to_string()
}

/// Pages: one level, `pages/<key>/<lang>.md`.
fn load_pages(dir: &Path, cfg: &Config, drafts: bool) -> Result<Vec<Group>> {
    let mut groups = Vec::new();
    if !dir.exists() {
        return Ok(groups);
    }
    for path in sorted_subdirs(dir)? {
        if let Some(g) = load_group(&path, &file_name(&path), "", cfg, drafts, None)? {
            groups.push(g);
        }
    }
    Ok(groups)
}

/// Posts: walks `posts/` recursively. A directory holding `<lang>.md` files is
/// a post; any other directory is a folder whose path prefixes its posts.
fn walk_posts(
    path: &Path,
    rel: &str,
    cfg: &Config,
    drafts: bool,
    posts: &mut Vec<Group>,
    dirs: &mut BTreeMap<String, DirMeta>,
) -> Result<()> {
    for sub in sorted_subdirs(path)? {
        let name = file_name(&sub);
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        let is_post = fs::read_dir(&sub)?
            .filter_map(|e| e.ok())
            .any(|e| is_markdown(&e.path()));
        if is_post {
            let slug_guess = base_slug(&name);
            let asset_base = format!("/posts/{}/", Group::path_of(rel, &slug_guess));
            if let Some(g) = load_group(&sub, &name, rel, cfg, drafts, Some(&asset_base))? {
                posts.push(g);
            }
            continue;
        }
        let child = Group::path_of(rel, &name);
        let meta_path = sub.join("_dir.toml");
        let meta: DirMeta = if meta_path.exists() {
            let raw = fs::read_to_string(&meta_path)?;
            toml::from_str(&raw).with_context(|| format!("{}: invalid folder metadata", meta_path.display()))?
        } else {
            DirMeta::default()
        };
        if child.contains(' ') {
            bail!("{}: folder names must not contain spaces", sub.display());
        }
        dirs.insert(child.clone(), meta);
        walk_posts(&sub, &child, cfg, drafts, posts, dirs)?;
    }
    Ok(())
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

    let mut pages = load_pages(&content.join("pages"), cfg, drafts)?;
    pages.sort_by(|a, b| {
        let w = |g: &Group| g.docs.values().next().map(|d| d.meta.weight).unwrap_or(0);
        w(a).cmp(&w(b)).then_with(|| a.key.cmp(&b.key))
    });

    let mut posts = Vec::new();
    let mut dirs = BTreeMap::new();
    let posts_dir = content.join("posts");
    if posts_dir.exists() {
        walk_posts(&posts_dir, "", cfg, drafts, &mut posts, &mut dirs)?;
    }
    // Keep only folders that actually contain a (non-draft) post.
    dirs.retain(|path, _| {
        posts
            .iter()
            .any(|g| g.dir == *path || g.dir.starts_with(&format!("{path}/")))
    });
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

    Ok(Content {
        home,
        pages,
        posts,
        dirs,
    })
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
    fn joins_folder_paths() {
        assert_eq!(Group::path_of("", "libft"), "libft");
        assert_eq!(Group::path_of("unix/bash", "sed"), "unix/bash/sed");
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
