//! Turns loaded content into a static site in `public/`.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::json;
use tera::{Context, Tera};
use walkdir::WalkDir;

use crate::config::{self, Config, Strings};
use crate::content::{self, Content, Doc, Group};
use crate::markdown::{escape_html, slugify, TocEntry};

pub struct BuildOptions {
    pub root: PathBuf,
    pub out: PathBuf,
    pub drafts: bool,
    /// Overrides `base_url` (used by `serve`).
    pub base_url: Option<String>,
    /// Injects the live-reload script.
    pub dev: bool,
}

pub struct BuildStats {
    pub pages: usize,
    pub posts: usize,
}

pub fn build(opts: &BuildOptions) -> Result<BuildStats> {
    let mut cfg = Config::load(&opts.root)?;
    if let Some(url) = &opts.base_url {
        cfg.base_url = url.trim_end_matches('/').to_string();
    }
    let strings = config::load_strings(&opts.root, &cfg)?;
    let content = content::load(&opts.root, &cfg, opts.drafts)?;
    let mut tera = Tera::new(&format!("{}/templates/**/*", opts.root.display()))
        .context("loading templates")?;
    // Tera's default escaper also encodes `/`, which makes URLs unreadable.
    tera.set_escape_fn(escape_html);

    prepare_out_dir(&opts.root, &opts.out)?;
    let site = Site {
        cfg,
        strings,
        content,
        tera,
        out: opts.out.clone(),
        dev: opts.dev,
        written: RefCell::default(),
        sitemap: RefCell::default(),
    };
    site.render_all()?;
    site.copy_files(&opts.root)?;

    let pages = site.written.borrow().len();
    Ok(BuildStats {
        pages,
        posts: site.content.posts.len(),
    })
}

fn prepare_out_dir(root: &Path, out: &Path) -> Result<()> {
    let root = root.canonicalize()?;
    if out.exists() {
        let abs = out.canonicalize()?;
        if root.starts_with(&abs) {
            bail!(
                "refusing to clear output directory {} (it contains the project)",
                abs.display()
            );
        }
        fs::remove_dir_all(&abs).with_context(|| format!("clearing {}", abs.display()))?;
    }
    fs::create_dir_all(out)?;
    Ok(())
}

// ─── View models handed to templates ─────────────────────────────────────────

#[derive(Serialize, Clone)]
struct LangLink {
    code: String,
    name: String,
    url: String,
    active: bool,
    /// False when the current page has no translation in this language
    /// (the link then points to that language's home page).
    exists: bool,
}

#[derive(Serialize)]
struct Alternate {
    hreflang: String,
    href: String,
}

#[derive(Serialize)]
struct NavItem {
    key: String,
    title: String,
    url: String,
}

#[derive(Serialize, Clone)]
struct TagLink {
    name: String,
    url: String,
    count: usize,
}

#[derive(Serialize, Clone)]
struct PostView {
    key: String,
    lang: String,
    lang_name: String,
    title: String,
    description: String,
    url: String,
    date_iso: String,
    date_display: String,
    updated_iso: Option<String>,
    updated_display: Option<String>,
    /// Localized, e.g. "3 min read" / "3분 읽기".
    reading_time: String,
    tags: Vec<TagLink>,
    /// Shown in a listing of another language because it is not translated.
    is_fallback: bool,
}

#[derive(Serialize)]
struct PageMeta {
    /// Full `<title>`.
    title: String,
    /// Title without the site name suffix (Open Graph).
    og_title: String,
    description: String,
    canonical: String,
    og_type: &'static str,
    og_image: String,
    json_ld: String,
    has_mermaid: bool,
    noindex: bool,
}

struct SitemapEntry {
    loc: String,
    lastmod: Option<NaiveDate>,
    alternates: Vec<(String, String)>,
}

// ─── Builder ─────────────────────────────────────────────────────────────────

struct Site {
    cfg: Config,
    strings: BTreeMap<String, Strings>,
    content: Content,
    tera: Tera,
    out: PathBuf,
    dev: bool,
    written: RefCell<HashSet<String>>,
    sitemap: RefCell<Vec<SitemapEntry>>,
}

/// `lang → url path` of the translations of the page being rendered.
type Alternates = BTreeMap<String, String>;

impl Site {
    fn render_all(&self) -> Result<()> {
        let langs = self.cfg.languages.clone();
        for lang in &langs {
            self.render_home(lang)?;
            self.render_post_list(lang)?;
            self.render_tags(lang)?;
            self.render_feed(lang)?;
        }
        for gi in 0..self.content.posts.len() {
            for lang in self.content.posts[gi]
                .docs
                .keys()
                .cloned()
                .collect::<Vec<_>>()
            {
                self.render_post(gi, &lang)?;
            }
        }
        for gi in 0..self.content.pages.len() {
            for lang in self.content.pages[gi]
                .docs
                .keys()
                .cloned()
                .collect::<Vec<_>>()
            {
                self.render_page(gi, &lang)?;
            }
        }
        self.render_404()?;
        self.write_sitemap()?;
        self.write_robots()?;
        Ok(())
    }

    // ── URLs ──

    fn home_url(&self, lang: &str) -> String {
        format!("{}/", self.cfg.prefix(lang))
    }
    fn posts_url(&self, lang: &str) -> String {
        format!("{}/posts/", self.cfg.prefix(lang))
    }
    fn tags_url(&self, lang: &str) -> String {
        format!("{}/tags/", self.cfg.prefix(lang))
    }
    fn tag_url(&self, lang: &str, tag: &str) -> String {
        encode_path(&format!("{}/tags/{}/", self.cfg.prefix(lang), slugify(tag)))
    }
    fn post_url(&self, doc: &Doc) -> String {
        encode_path(&format!(
            "{}/posts/{}/",
            self.cfg.prefix(&doc.lang),
            doc.slug
        ))
    }
    fn page_url(&self, doc: &Doc) -> String {
        encode_path(&format!("{}/{}/", self.cfg.prefix(&doc.lang), doc.slug))
    }
    fn feed_url(&self, lang: &str) -> String {
        format!("{}/atom.xml", self.cfg.prefix(lang))
    }
    fn abs(&self, path: &str) -> String {
        if path.contains("://") {
            path.to_string()
        } else {
            format!("{}{}", self.cfg.base_url, path)
        }
    }

    fn t(&self, lang: &str) -> &Strings {
        &self.strings[lang]
    }

    /// Alternates for pages that exist in every language (home, lists…).
    fn all_langs(&self, f: impl Fn(&Self, &str) -> String) -> Alternates {
        self.cfg
            .languages
            .iter()
            .map(|l| (l.clone(), f(self, l)))
            .collect()
    }

    // ── Views ──

    fn post_view(&self, doc: &Doc, viewer_lang: &str) -> PostView {
        let t = self.t(&doc.lang);
        let date = doc.date.expect("posts always have a date");
        PostView {
            key: doc.key.clone(),
            lang: doc.lang.clone(),
            lang_name: t.language_name.clone(),
            title: doc.meta.title.clone(),
            description: doc.description().to_string(),
            url: self.post_url(doc),
            date_iso: date.format("%Y-%m-%d").to_string(),
            date_display: self.t(viewer_lang).format_date(date),
            updated_iso: doc.updated.map(|d| d.format("%Y-%m-%d").to_string()),
            updated_display: doc.updated.map(|d| self.t(viewer_lang).format_date(d)),
            reading_time: self.t(viewer_lang).get("reading_time").replace(
                "{n}",
                &doc.reading_minutes(self.cfg.words_per_minute).to_string(),
            ),
            tags: doc
                .meta
                .tags
                .iter()
                .map(|tag| TagLink {
                    name: tag.clone(),
                    url: self.tag_url(viewer_lang, tag),
                    count: 0,
                })
                .collect(),
            is_fallback: doc.lang != viewer_lang,
        }
    }

    /// Posts listed for `lang`: its own posts plus (optionally) untranslated ones.
    fn listing(&self, lang: &str) -> Vec<&Doc> {
        self.content
            .posts
            .iter()
            .filter_map(|g| match g.docs.get(lang) {
                Some(d) => Some(d),
                None if self.cfg.show_untranslated => g.best(lang, &self.cfg),
                None => None,
            })
            .collect()
    }

    /// tag slug → (display name, posts) for `lang`.
    fn tag_index(&self, lang: &str) -> BTreeMap<String, (String, Vec<&Doc>)> {
        let mut map: BTreeMap<String, (String, Vec<&Doc>)> = BTreeMap::new();
        for doc in self.listing(lang) {
            for tag in &doc.meta.tags {
                map.entry(slugify(tag))
                    .or_insert_with(|| (tag.clone(), Vec::new()))
                    .1
                    .push(doc);
            }
        }
        map
    }

    fn base_context(&self, lang: &str, section: &str, alternates: &Alternates) -> Context {
        let t = self.t(lang);
        let mut ctx = Context::new();
        ctx.insert("site", &self.cfg);
        ctx.insert("lang", lang);
        ctx.insert("t", &t.table);
        ctx.insert("section", section);
        ctx.insert("dev", &self.dev);
        ctx.insert("comments_enabled", &self.cfg.comments.enabled());
        ctx.insert("home_url", &self.home_url(lang));
        ctx.insert("feed_url", &self.feed_url(lang));
        ctx.insert(
            "year",
            &chrono::Datelike::year(&chrono::Local::now().date_naive()),
        );

        let languages: Vec<LangLink> = self
            .cfg
            .languages
            .iter()
            .map(|code| LangLink {
                code: code.clone(),
                name: self.t(code).language_name.clone(),
                url: alternates
                    .get(code)
                    .cloned()
                    .unwrap_or_else(|| self.home_url(code)),
                active: code == lang,
                exists: alternates.contains_key(code),
            })
            .collect();
        ctx.insert("languages", &languages);

        let mut alts: Vec<Alternate> = alternates
            .iter()
            .map(|(l, u)| Alternate {
                hreflang: l.clone(),
                href: self.abs(u),
            })
            .collect();
        let x_default = alternates.get(&self.cfg.default_language).or_else(|| {
            self.cfg
                .fallback_order()
                .iter()
                .find_map(|l| alternates.get(*l))
        });
        if let Some(u) = x_default {
            alts.push(Alternate {
                hreflang: "x-default".into(),
                href: self.abs(u),
            });
        }
        ctx.insert("alternates", &alts);
        let og_alt_locales: Vec<&str> = alternates
            .keys()
            .filter(|l| *l != lang)
            .map(|l| self.t(l).locale.as_str())
            .collect();
        ctx.insert("og_locale", &t.locale);
        ctx.insert("og_locale_alternates", &og_alt_locales);

        let mut nav = vec![
            NavItem {
                key: "home".into(),
                title: t.get("nav_home").into(),
                url: self.home_url(lang),
            },
            NavItem {
                key: "posts".into(),
                title: t.get("nav_posts").into(),
                url: self.posts_url(lang),
            },
            NavItem {
                key: "tags".into(),
                title: t.get("nav_tags").into(),
                url: self.tags_url(lang),
            },
        ];
        for g in &self.content.pages {
            if let Some(doc) = g.best(lang, &self.cfg).filter(|d| d.meta.in_menu) {
                nav.push(NavItem {
                    key: g.key.clone(),
                    title: doc.meta.title.clone(),
                    url: self.page_url(doc),
                });
            }
        }
        ctx.insert("nav", &nav);
        ctx
    }

    fn page_meta(&self, lang: &str, title: Option<&str>, description: &str, url: &str) -> PageMeta {
        let site_title = &self.t(lang).site_title;
        PageMeta {
            title: match title {
                Some(t) => format!("{t} | {site_title}"),
                None => site_title.clone(),
            },
            og_title: title.unwrap_or(site_title).to_string(),
            description: description.to_string(),
            canonical: self.abs(url),
            og_type: "website",
            og_image: self.abs(&self.cfg.seo.og_image),
            json_ld: String::new(),
            has_mermaid: false,
            noindex: false,
        }
    }

    fn person_ld(&self) -> serde_json::Value {
        let a = &self.cfg.author;
        let mut same_as = Vec::new();
        if !a.github.is_empty() {
            same_as.push(format!("https://github.com/{}", a.github));
        }
        if !a.linkedin.is_empty() {
            same_as.push(a.linkedin.clone());
        }
        if !a.twitter.is_empty() {
            same_as.push(format!("https://x.com/{}", a.twitter));
        }
        json!({ "@type": "Person", "name": a.name, "url": self.abs("/"), "sameAs": same_as })
    }

    // ── Rendering ──

    fn render_home(&self, lang: &str) -> Result<()> {
        let url = self.home_url(lang);
        let alternates = self.all_langs(Self::home_url);
        let mut ctx = self.base_context(lang, "home", &alternates);
        let t = self.t(lang);

        let mut meta = self.page_meta(lang, None, &t.site_description, &url);
        meta.json_ld = ld(json!({
            "@context": "https://schema.org",
            "@graph": [
                { "@type": "WebSite", "name": t.site_title, "url": self.abs(&url), "inLanguage": lang, "description": t.site_description },
                self.person_ld(),
            ]
        }));
        ctx.insert("page", &meta);

        let bio = self
            .content
            .home
            .get(lang)
            .or_else(|| {
                self.cfg
                    .fallback_order()
                    .iter()
                    .find_map(|l| self.content.home.get(*l))
            })
            .map(|r| r.html.clone())
            .unwrap_or_default();
        ctx.insert("bio", &bio);
        let recent: Vec<PostView> = self
            .listing(lang)
            .into_iter()
            .take(5)
            .map(|d| self.post_view(d, lang))
            .collect();
        ctx.insert("recent_posts", &recent);
        ctx.insert("posts_url", &self.posts_url(lang));
        let mut tags: Vec<TagLink> = self
            .tag_index(lang)
            .values()
            .map(|(name, docs)| TagLink {
                name: name.clone(),
                url: self.tag_url(lang, name),
                count: docs.len(),
            })
            .collect();
        tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        tags.truncate(12);
        ctx.insert("top_tags", &tags);

        self.render_to("home.html", &ctx, &url)?;
        self.add_sitemap(&url, None, &alternates);
        Ok(())
    }

    fn render_post_list(&self, lang: &str) -> Result<()> {
        let url = self.posts_url(lang);
        let alternates = self.all_langs(Self::posts_url);
        let mut ctx = self.base_context(lang, "posts", &alternates);
        let t = self.t(lang);
        let title = t.get("all_posts").to_string();
        let meta = self.page_meta(lang, Some(&title), &t.site_description, &url);
        ctx.insert("page", &meta);
        ctx.insert("heading", &title);
        let posts: Vec<PostView> = self
            .listing(lang)
            .into_iter()
            .map(|d| self.post_view(d, lang))
            .collect();
        ctx.insert("posts", &posts);
        self.render_to("posts.html", &ctx, &url)?;
        self.add_sitemap(&url, None, &alternates);
        Ok(())
    }

    fn render_tags(&self, lang: &str) -> Result<()> {
        // Tag index.
        let url = self.tags_url(lang);
        let alternates = self.all_langs(Self::tags_url);
        let mut ctx = self.base_context(lang, "tags", &alternates);
        let t = self.t(lang);
        let title = t.get("tags").to_string();
        ctx.insert(
            "page",
            &self.page_meta(lang, Some(&title), &t.site_description, &url),
        );
        let index = self.tag_index(lang);
        let tags: Vec<TagLink> = index
            .values()
            .map(|(name, docs)| TagLink {
                name: name.clone(),
                url: self.tag_url(lang, name),
                count: docs.len(),
            })
            .collect();
        ctx.insert("tags", &tags);
        self.render_to("tags.html", &ctx, &url)?;
        self.add_sitemap(&url, None, &alternates);

        // One page per tag; alternates = languages where the same tag exists.
        let per_lang: BTreeMap<String, HashSet<String>> = self
            .cfg
            .languages
            .iter()
            .map(|l| (l.clone(), self.tag_index(l).into_keys().collect()))
            .collect();
        for (slug, (name, docs)) in &index {
            let url = self.tag_url(lang, name);
            let alternates: Alternates = per_lang
                .iter()
                .filter(|(_, set)| set.contains(slug))
                .map(|(l, _)| (l.clone(), self.tag_url(l, name)))
                .collect();
            let mut ctx = self.base_context(lang, "tags", &alternates);
            let heading = format!("{} #{}", t.get("tagged"), name);
            let mut meta = self.page_meta(lang, Some(&format!("#{name}")), &heading, &url);
            // Thin listing pages: keep them crawlable but out of the index.
            meta.noindex = true;
            ctx.insert("page", &meta);
            ctx.insert("heading", &heading);
            ctx.insert("tag", name);
            let posts: Vec<PostView> = docs.iter().map(|d| self.post_view(d, lang)).collect();
            ctx.insert("posts", &posts);
            self.render_to("posts.html", &ctx, &url)?;
        }
        Ok(())
    }

    fn render_post(&self, gi: usize, lang: &str) -> Result<()> {
        let group = &self.content.posts[gi];
        let doc = &group.docs[lang];
        let url = self.post_url(doc);
        let alternates: Alternates = group
            .docs
            .iter()
            .map(|(l, d)| (l.clone(), self.post_url(d)))
            .collect();
        let mut ctx = self.base_context(lang, "posts", &alternates);
        let view = self.post_view(doc, lang);

        let mut meta = self.page_meta(lang, Some(&doc.meta.title), doc.description(), &url);
        meta.og_type = "article";
        meta.has_mermaid = doc.rendered.has_mermaid;
        if let Some(img) = &doc.meta.image {
            meta.og_image = self.abs(img);
        }
        let t = self.t(lang);
        meta.json_ld = ld(json!({
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "BlogPosting",
                    "headline": doc.meta.title,
                    "description": doc.description(),
                    "datePublished": view.date_iso,
                    "dateModified": view.updated_iso.clone().unwrap_or_else(|| view.date_iso.clone()),
                    "inLanguage": lang,
                    "url": self.abs(&url),
                    "mainEntityOfPage": self.abs(&url),
                    "image": meta.og_image,
                    "keywords": doc.meta.tags,
                    "wordCount": doc.rendered.word_count,
                    "author": self.person_ld(),
                },
                {
                    "@type": "BreadcrumbList",
                    "itemListElement": [
                        { "@type": "ListItem", "position": 1, "name": t.get("nav_home"), "item": self.abs(&self.home_url(lang)) },
                        { "@type": "ListItem", "position": 2, "name": t.get("nav_posts"), "item": self.abs(&self.posts_url(lang)) },
                        { "@type": "ListItem", "position": 3, "name": doc.meta.title, "item": self.abs(&url) },
                    ]
                }
            ]
        }));
        ctx.insert("page", &meta);
        ctx.insert("post", &view);
        ctx.insert("content", &doc.rendered.html);
        let toc: &[TocEntry] = if doc.meta.toc && doc.rendered.toc.len() >= 2 {
            &doc.rendered.toc
        } else {
            &[]
        };
        ctx.insert("toc", toc);
        ctx.insert(
            "show_comments",
            &(doc.meta.comments && self.cfg.comments.enabled()),
        );
        // All translations share one discussion thread.
        ctx.insert("comment_term", &content::base_slug(&group.key));

        let translations: Vec<PostView> = group
            .docs
            .values()
            .filter(|d| d.lang != lang)
            .map(|d| self.post_view(d, lang))
            .collect();
        ctx.insert("translations", &translations);

        // Previous / next within the same language.
        let own: Vec<&Doc> = self
            .content
            .posts
            .iter()
            .filter_map(|g| g.docs.get(lang))
            .collect();
        if let Some(pos) = own.iter().position(|d| d.key == doc.key) {
            ctx.insert(
                "newer",
                &pos.checked_sub(1).map(|i| self.post_view(own[i], lang)),
            );
            ctx.insert("older", &own.get(pos + 1).map(|d| self.post_view(d, lang)));
        }

        let lastmod = doc.updated.or(doc.date);
        self.render_to("post.html", &ctx, &url)?;
        self.add_sitemap(&url, lastmod, &alternates);
        Ok(())
    }

    fn render_page(&self, gi: usize, lang: &str) -> Result<()> {
        let group: &Group = &self.content.pages[gi];
        let doc = &group.docs[lang];
        let url = self.page_url(doc);
        let alternates: Alternates = group
            .docs
            .iter()
            .map(|(l, d)| (l.clone(), self.page_url(d)))
            .collect();
        let mut ctx = self.base_context(lang, &group.key, &alternates);
        let mut meta = self.page_meta(lang, Some(&doc.meta.title), doc.description(), &url);
        meta.has_mermaid = doc.rendered.has_mermaid;
        meta.json_ld = ld(json!({
            "@context": "https://schema.org",
            "@type": if group.key == "about" { "ProfilePage" } else { "WebPage" },
            "name": doc.meta.title,
            "url": self.abs(&url),
            "inLanguage": lang,
            "mainEntity": self.person_ld(),
        }));
        ctx.insert("page", &meta);
        ctx.insert("title", &doc.meta.title);
        ctx.insert("content", &doc.rendered.html);
        let lastmod = doc.updated.or(doc.date);
        self.render_to("page.html", &ctx, &url)?;
        self.add_sitemap(&url, lastmod, &alternates);
        Ok(())
    }

    fn render_404(&self) -> Result<()> {
        let lang = self.cfg.default_language.clone();
        let alternates = Alternates::new();
        let mut ctx = self.base_context(&lang, "404", &alternates);
        let t = self.t(&lang);
        let mut meta = self.page_meta(
            &lang,
            Some(t.get("not_found_title")),
            t.get("not_found_body"),
            "/404.html",
        );
        meta.noindex = true;
        ctx.insert("page", &meta);
        let homes: Vec<LangLink> = self
            .cfg
            .languages
            .iter()
            .map(|l| LangLink {
                code: l.clone(),
                name: self.t(l).language_name.clone(),
                url: self.home_url(l),
                active: *l == lang,
                exists: true,
            })
            .collect();
        ctx.insert("homes", &homes);
        let html = self
            .tera
            .render("404.html", &ctx)
            .context("rendering 404.html")?;
        self.write_file("404.html", &html)
    }

    fn render_feed(&self, lang: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Entry {
            title: String,
            url: String,
            published: String,
            updated: String,
            summary: String,
            content: String,
            tags: Vec<String>,
        }
        let rfc = |d: NaiveDate| format!("{}T00:00:00Z", d.format("%Y-%m-%d"));
        let own: Vec<&Doc> = self
            .content
            .posts
            .iter()
            .filter_map(|g| g.docs.get(lang))
            .take(20)
            .collect();
        let entries: Vec<Entry> = own
            .iter()
            .map(|d| {
                let date = d.date.expect("posts have dates");
                Entry {
                    title: d.meta.title.clone(),
                    url: self.abs(&self.post_url(d)),
                    published: rfc(date),
                    updated: rfc(d.updated.unwrap_or(date)),
                    summary: d.description().to_string(),
                    content: d.rendered.html.clone(),
                    tags: d.meta.tags.clone(),
                }
            })
            .collect();
        let updated = own
            .iter()
            .filter_map(|d| d.updated.or(d.date))
            .max()
            .map(rfc)
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".into());

        let t = self.t(lang);
        let mut ctx = Context::new();
        ctx.insert("site", &self.cfg);
        ctx.insert("lang", lang);
        ctx.insert("title", &t.site_title);
        ctx.insert("subtitle", &t.site_description);
        ctx.insert("feed_url", &self.abs(&self.feed_url(lang)));
        ctx.insert("home_url", &self.abs(&self.home_url(lang)));
        ctx.insert("updated", &updated);
        ctx.insert("entries", &entries);
        let xml = self
            .tera
            .render("atom.xml", &ctx)
            .context("rendering atom.xml")?;
        let path = self.feed_url(lang);
        self.write_file(path.trim_start_matches('/'), &xml)
    }

    fn add_sitemap(&self, url: &str, lastmod: Option<NaiveDate>, alternates: &Alternates) {
        let alternates = if alternates.len() > 1 {
            alternates
                .iter()
                .map(|(l, u)| (l.clone(), self.abs(u)))
                .collect()
        } else {
            Vec::new()
        };
        self.sitemap.borrow_mut().push(SitemapEntry {
            loc: self.abs(url),
            lastmod,
            alternates,
        });
    }

    fn write_sitemap(&self) -> Result<()> {
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
        );
        for e in self.sitemap.borrow().iter() {
            xml.push_str(&format!(
                "  <url>\n    <loc>{}</loc>\n",
                escape_html(&e.loc)
            ));
            if let Some(d) = e.lastmod {
                xml.push_str(&format!(
                    "    <lastmod>{}</lastmod>\n",
                    d.format("%Y-%m-%d")
                ));
            }
            for (l, href) in &e.alternates {
                xml.push_str(&format!(
                    "    <xhtml:link rel=\"alternate\" hreflang=\"{l}\" href=\"{}\"/>\n",
                    escape_html(href)
                ));
            }
            xml.push_str("  </url>\n");
        }
        xml.push_str("</urlset>\n");
        self.write_file("sitemap.xml", &xml)
    }

    fn write_robots(&self) -> Result<()> {
        let robots = format!(
            "User-agent: *\nAllow: /\n\nSitemap: {}\n",
            self.abs("/sitemap.xml")
        );
        self.write_file("robots.txt", &robots)
    }

    // ── Output ──

    /// Renders `template` to `<url>/index.html`.
    fn render_to(&self, template: &str, ctx: &Context, url: &str) -> Result<()> {
        let html = self
            .tera
            .render(template, ctx)
            .with_context(|| format!("rendering {template} for {url}"))?;
        let path = format!("{}index.html", decode_path(url).trim_start_matches('/'));
        self.write_file(&path, &html)
    }

    fn write_file(&self, rel: &str, contents: &str) -> Result<()> {
        if !self.written.borrow_mut().insert(rel.to_string()) {
            bail!("two pages want to be written to `{rel}` — check for duplicate slugs");
        }
        let path = self.out.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))
    }

    fn copy_files(&self, root: &Path) -> Result<()> {
        // static/ → public/
        let static_dir = root.join("static");
        if static_dir.exists() {
            for entry in WalkDir::new(&static_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let rel = entry.path().strip_prefix(&static_dir)?;
                let dest = self.out.join(rel);
                fs::create_dir_all(dest.parent().unwrap())?;
                fs::copy(entry.path(), &dest)?;
            }
        }
        // Post assets → /posts/<slug>/
        for g in &self.content.posts {
            if g.assets.is_empty() {
                continue;
            }
            let dir = self.out.join("posts").join(content::base_slug(&g.key));
            fs::create_dir_all(&dir)?;
            for a in &g.assets {
                fs::copy(a, dir.join(a.file_name().unwrap()))?;
            }
        }
        fs::create_dir_all(self.out.join("css"))?;
        fs::write(
            self.out.join("css/syntax.css"),
            crate::markdown::syntax_css()?,
        )?;
        fs::write(self.out.join(".nojekyll"), "")?;
        Ok(())
    }
}

/// Serializes JSON-LD so it is safe inside a `<script>` element.
fn ld(v: serde_json::Value) -> String {
    v.to_string().replace("</", "<\\/")
}

/// Percent-encodes a URL path (keeps `/`), so non-ASCII slugs become valid URLs.
fn encode_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode_path(p: &str) -> String {
    let bytes = p.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = p
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_encoding_roundtrip() {
        let p = "/ko/tags/러스트/";
        let enc = encode_path(p);
        assert_eq!(enc, "/ko/tags/%EB%9F%AC%EC%8A%A4%ED%8A%B8/");
        assert_eq!(decode_path(&enc), p);
    }
}
