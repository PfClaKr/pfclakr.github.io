//! Markdown → HTML rendering.
//!
//! On top of CommonMark/GFM (tables, footnotes, task lists, alerts…) this adds:
//! - build-time syntax highlighting (syntect, CSS classes → no JS needed)
//! - ```mermaid code blocks rendered as `<pre class="mermaid">` for mermaid.js
//! - heading anchors + a table of contents
//! - lazy-loaded images, with relative paths rewritten to the post's asset folder
//! - admonitions: Docusaurus-style `:::warning Title` … `:::` blocks and GitHub
//!   `> [!WARNING]` alerts, both rendered as `<div class="admonition …">` with a
//!   localized default title

use std::collections::HashSet;
use std::sync::OnceLock;

use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use serde::Serialize;
use syntect::highlighting::Theme;
use syntect::html::{css_for_theme_with_class_style, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

#[derive(Debug, Clone, Serialize)]
pub struct TocEntry {
    pub level: u8,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Default)]
pub struct Rendered {
    pub html: String,
    pub toc: Vec<TocEntry>,
    pub has_mermaid: bool,
    pub word_count: usize,
    /// Plain text of the first paragraph, used as a fallback meta description.
    pub summary: String,
    /// Plain text of the whole document (code blocks excluded), for search.
    pub text: String,
}

/// Renders markdown with English admonition titles (tests).
#[cfg(test)]
pub fn render(markdown: &str, asset_base: Option<&str>) -> Rendered {
    render_lang(markdown, asset_base, "en")
}

/// Renders markdown. `asset_base` (e.g. `/posts/hello/`) is prepended to relative image URLs;
/// admonitions get their default titles in `lang`.
pub fn render_lang(markdown: &str, asset_base: Option<&str>, lang: &str) -> Rendered {
    let markdown = expand_admonitions(markdown, lang);
    let markdown = markdown.as_str();
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    opts.insert(Options::ENABLE_GFM);

    let mut out = Rendered::default();
    let mut events: Vec<Event> = Vec::new();
    let mut used_ids: HashSet<String> = HashSet::new();
    let mut parser = Parser::new_ext(markdown, opts);

    let mut in_first_paragraph = false;
    let mut summary_done = false;

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match &kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                let mut code = String::new();
                for ev in parser.by_ref() {
                    match ev {
                        Event::Text(t) => code.push_str(&t),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }
                if lang == "mermaid" {
                    out.has_mermaid = true;
                    events.push(Event::Html(
                        format!("<pre class=\"mermaid\">{}</pre>\n", escape_html(&code)).into(),
                    ));
                } else {
                    events.push(Event::Html(highlight(&code, &lang).into()));
                }
            }

            Event::Start(Tag::Heading {
                level,
                id,
                classes,
                attrs,
            }) => {
                let mut inner = Vec::new();
                let mut text = String::new();
                for ev in parser.by_ref() {
                    match ev {
                        Event::End(TagEnd::Heading(_)) => break,
                        Event::Text(ref t) | Event::Code(ref t) => {
                            text.push_str(t);
                            inner.push(ev);
                        }
                        other => inner.push(other),
                    }
                }
                let base = id.map(|s| s.to_string()).unwrap_or_else(|| slugify(&text));
                let id = unique_id(
                    if base.is_empty() {
                        "section".into()
                    } else {
                        base
                    },
                    &mut used_ids,
                );
                out.text.push_str(&text);
                out.text.push(' ');
                let lvl = heading_level(level);
                if (2..=3).contains(&lvl) {
                    out.toc.push(TocEntry {
                        level: lvl,
                        id: id.clone(),
                        title: text.trim().to_string(),
                    });
                }
                let mut inner_html = String::new();
                pulldown_cmark::html::push_html(&mut inner_html, inner.into_iter());
                let class_attr = if classes.is_empty() {
                    String::new()
                } else {
                    format!(
                        " class=\"{}\"",
                        escape_html(
                            &classes
                                .iter()
                                .map(|c| c.as_ref())
                                .collect::<Vec<_>>()
                                .join(" ")
                        )
                    )
                };
                let extra_attrs: String = attrs
                    .iter()
                    .map(|(k, v)| match v {
                        Some(v) => format!(" {}=\"{}\"", escape_html(k), escape_html(v)),
                        None => format!(" {}", escape_html(k)),
                    })
                    .collect();
                events.push(Event::Html(
                    format!(
                        "<h{lvl} id=\"{id}\"{class_attr}{extra_attrs}>{inner_html} <a class=\"anchor\" href=\"#{id}\" aria-hidden=\"true\" tabindex=\"-1\">#</a></h{lvl}>\n",
                        id = escape_html(&id),
                    )
                    .into(),
                ));
            }

            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                let mut alt = String::new();
                for ev in parser.by_ref() {
                    match ev {
                        Event::End(TagEnd::Image) => break,
                        Event::Text(t) | Event::Code(t) => alt.push_str(&t),
                        _ => {}
                    }
                }
                let src = resolve_asset(&dest_url, asset_base);
                let title_attr = if title.is_empty() {
                    String::new()
                } else {
                    format!(" title=\"{}\"", escape_html(&title))
                };
                events.push(Event::Html(
                    format!(
                        "<img src=\"{}\" alt=\"{}\"{} loading=\"lazy\" decoding=\"async\">",
                        escape_html(&src),
                        escape_html(&alt),
                        title_attr
                    )
                    .into(),
                ));
            }

            // GitHub alerts (`> [!NOTE]`) get the same markup as `:::note`.
            Event::Start(Tag::BlockQuote(Some(kind))) => {
                let kind = match kind {
                    BlockQuoteKind::Note => "note",
                    BlockQuoteKind::Tip => "tip",
                    BlockQuoteKind::Important => "important",
                    BlockQuoteKind::Warning => "warning",
                    BlockQuoteKind::Caution => "danger",
                };
                events.push(Event::Html(admonition_open(kind, "", lang).into()));
            }
            Event::End(TagEnd::BlockQuote(Some(_))) => {
                out.text.push(' ');
                events.push(Event::Html("</div>\n".into()));
            }

            Event::Start(Tag::Paragraph) => {
                if !summary_done {
                    in_first_paragraph = true;
                }
                events.push(Event::Start(Tag::Paragraph));
            }
            Event::End(TagEnd::Paragraph) => {
                out.text.push(' ');
                if in_first_paragraph {
                    in_first_paragraph = false;
                    summary_done = !out.summary.trim().is_empty();
                }
                events.push(Event::End(TagEnd::Paragraph));
            }

            Event::Text(t) => {
                out.text.push_str(&t);
                out.word_count += t.split_whitespace().count();
                if in_first_paragraph {
                    out.summary.push_str(&t);
                }
                events.push(Event::Text(t));
            }
            Event::Code(t) => {
                out.text.push_str(&t);
                out.word_count += t.split_whitespace().count();
                if in_first_paragraph {
                    out.summary.push_str(&t);
                }
                events.push(Event::Code(t));
            }
            Event::SoftBreak | Event::HardBreak => {
                out.text.push(' ');
                if in_first_paragraph {
                    out.summary.push(' ');
                }
                events.push(event);
            }

            other => {
                // Block ends separate words; inline ends (links, emphasis) must not.
                let inline_end = matches!(
                    other,
                    Event::End(
                        TagEnd::Link | TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough
                    )
                );
                if matches!(other, Event::End(_)) && !inline_end {
                    out.text.push(' ');
                }
                events.push(other)
            }
        }
    }

    pulldown_cmark::html::push_html(&mut out.html, events.into_iter());
    out.text = out.text.split_whitespace().collect::<Vec<_>>().join(" ");
    out.summary = truncate_chars(
        out.summary
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .as_str(),
        160,
    );
    out
}

/// Admonition kinds and their default titles (ko, fr, en).
fn admonition_kind(name: &str) -> Option<(&'static str, [&'static str; 3])> {
    Some(match name {
        "note" => ("note", ["참고", "Remarque", "Note"]),
        "tip" => ("tip", ["팁", "Astuce", "Tip"]),
        "info" => ("info", ["정보", "Info", "Info"]),
        "important" => ("important", ["중요", "Important", "Important"]),
        // Docusaurus treats `caution` as an alias of `warning`.
        "warning" | "caution" => ("warning", ["주의", "Attention", "Warning"]),
        "danger" => ("danger", ["위험", "Danger", "Danger"]),
        _ => return None,
    })
}

fn admonition_open(kind: &str, title: &str, lang: &str) -> String {
    let (kind, defaults) = admonition_kind(kind).unwrap_or(("note", ["참고", "Remarque", "Note"]));
    let title = if title.trim().is_empty() {
        match lang {
            "ko" => defaults[0],
            "fr" => defaults[1],
            _ => defaults[2],
        }
    } else {
        title.trim()
    };
    format!(
        "<div class=\"admonition admonition-{kind}\">\n<p class=\"admonition-title\">{}</p>\n",
        escape_html(title)
    )
}

/// Turns `:::kind [title]` … `:::` blocks into an HTML wrapper around normal
/// markdown. Only lines starting at column 0 count, and fenced code is left
/// alone. `:::kind[title]` (Docusaurus v3) works too. Blocks can nest.
fn expand_admonitions(markdown: &str, lang: &str) -> String {
    let mut out = String::with_capacity(markdown.len() + 64);
    let mut fence: Option<String> = None;
    let mut depth = 0usize;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_end();
        let lead = trimmed.trim_start();
        if let Some(f) = &fence {
            if lead.starts_with(f.as_str()) && lead.trim_start_matches(f.chars().next().unwrap()).trim().is_empty() {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if lead.starts_with("```") || lead.starts_with("~~~") {
            let ch = lead.chars().next().unwrap();
            let n = lead.chars().take_while(|&c| c == ch).count();
            fence = Some(ch.to_string().repeat(n));
            out.push_str(line);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(":::") {
            if rest.trim().is_empty() && depth > 0 {
                depth -= 1;
                out.push_str("\n</div>\n\n");
                continue;
            }
            let name: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            if let Some((kind, _)) = admonition_kind(&name.to_ascii_lowercase()) {
                let after = &rest[name.len()..];
                let title = after
                    .trim()
                    .strip_prefix('[')
                    .and_then(|t| t.strip_suffix(']'))
                    .unwrap_or(after.trim());
                depth += 1;
                out.push('\n');
                out.push_str(&admonition_open(kind, title, lang));
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
    }
    for _ in 0..depth {
        out.push_str("\n</div>\n");
    }
    out
}

fn highlight(code: &str, lang: &str) -> String {
    let ss = syntax_set();
    let syntax = (!lang.is_empty())
        .then(|| ss.find_syntax_by_token(lang))
        .flatten()
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut gen = ClassedHTMLGenerator::new_with_class_style(syntax, ss, CLASS_STYLE);
    for line in LinesWithEndings::from(code) {
        if gen
            .parse_html_for_line_which_includes_newline(line)
            .is_err()
        {
            // Fall back to plain escaped text on grammar errors.
            return format!(
                "<pre class=\"code\"><code>{}</code></pre>\n",
                escape_html(code)
            );
        }
    }
    let lang_attr = if lang.is_empty() {
        String::new()
    } else {
        format!(" data-lang=\"{}\"", escape_html(lang))
    };
    format!(
        "<pre class=\"code hl-code\"{lang_attr}><code>{}</code></pre>\n",
        gen.finalize()
    )
}

/// CSS for highlighted code: a light theme by default and a dark one scoped
/// under `[data-theme="dark"]` (set by the theme toggle).
pub fn syntax_css() -> anyhow::Result<String> {
    use two_face::theme::EmbeddedThemeName as N;
    let themes = two_face::theme::extra();
    let light: &Theme = themes.get(N::OneHalfLight);
    let dark: &Theme = themes.get(N::OneHalfDark);
    let light_css = css_for_theme_with_class_style(light, CLASS_STYLE)?;
    let dark_css = css_for_theme_with_class_style(dark, CLASS_STYLE)?;
    Ok(format!(
        "/* Generated by the blog builder — do not edit. */\n{light_css}\n\
         [data-theme=\"dark\"] {{\n{dark_css}\n}}\n\
         @media (prefers-color-scheme: dark) {{\n  html:not([data-theme]) {{\n{dark_css}\n  }}\n}}\n"
    ))
}

fn resolve_asset(url: &CowStr, asset_base: Option<&str>) -> String {
    let u = url.as_ref();
    let is_relative =
        !(u.starts_with('/') || u.starts_with('#') || u.contains("://") || u.starts_with("data:"));
    match asset_base {
        Some(base) if is_relative => format!("{}{}", base, u.trim_start_matches("./")),
        _ => u.to_string(),
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn unique_id(base: String, used: &mut HashSet<String>) -> String {
    let mut id = base.clone();
    let mut n = 1;
    while !used.insert(id.clone()) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Unicode-aware slug: keeps letters/digits of any script (한글, accents…),
/// lowercases, and joins words with `-`.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            dash = false;
        } else if (c.is_whitespace() || c == '-' || c == '_' || c == '.' || c == '/')
            && !out.is_empty()
            && !dash
        {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admonitions_from_both_syntaxes() {
        let r = render_lang(
            ":::warning 주의할 점\n키 로그인을 **먼저** 확인합니다.\n:::\n\n:::tip\n팁.\n:::\n\n> [!NOTE]\n> 참고.\n",
            None,
            "ko",
        );
        assert!(r.html.contains("<div class=\"admonition admonition-warning\">"));
        assert!(r.html.contains("<p class=\"admonition-title\">주의할 점</p>"));
        assert!(r.html.contains("<strong>먼저</strong>"));
        assert!(r.html.contains("<p class=\"admonition-title\">팁</p>"));
        assert!(r.html.contains("<div class=\"admonition admonition-note\">\n<p class=\"admonition-title\">참고</p>"));
        assert!(!r.html.contains(":::"));
        assert!(!r.html.contains("<blockquote"));
        assert!(r.text.contains("키 로그인을 먼저 확인합니다."));
        assert_eq!(r.html.matches("<div class=\"admonition").count(), r.html.matches("</div>").count());
    }

    #[test]
    fn admonition_markers_inside_code_and_unknown_kinds_stay_text() {
        let r = render_lang("```\n:::note\n```\n\n:::nope\ntext\n:::\n\n:::caution[Mind the gap]\nx\n:::\n", None, "en");
        assert!(r.html.contains(":::note"));
        assert!(r.html.contains(":::nope"));
        assert!(r.html.contains("admonition-warning"));
        assert!(r.html.contains(">Mind the gap</p>"));
    }

    #[test]
    fn slugify_handles_scripts() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("러스트 소유권 이해하기"), "러스트-소유권-이해하기");
        assert_eq!(slugify("Élégant  café"), "élégant-café");
    }

    #[test]
    fn mermaid_and_code() {
        let r = render(
            "```mermaid\ngraph TD; A-->B\n```\n\n```rust\nfn main() {}\n```",
            None,
        );
        assert!(r.has_mermaid);
        assert!(r
            .html
            .contains("<pre class=\"mermaid\">graph TD; A--&gt;B\n</pre>"));
        assert!(r.html.contains("hl-code"));
    }

    #[test]
    fn headings_get_unique_ids() {
        let r = render("## Intro\n\n## Intro\n\n### Détails", None);
        let ids: Vec<_> = r.toc.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["intro", "intro-1", "détails"]);
    }

    #[test]
    fn relative_images_are_rewritten() {
        let r = render("![diagram](./arch.png)", Some("/posts/hello/"));
        assert!(r.html.contains("src=\"/posts/hello/arch.png\""));
        assert!(r.html.contains("loading=\"lazy\""));
    }

    #[test]
    fn plain_text_skips_code_blocks() {
        let r = render("# Title\n\nSome *text* and `code`, [링크](/x)의 뜻.\n\n```rust\nfn hidden() {}\n```\n\n- item one\n- item two\n", None);
        assert_eq!(r.text, "Title Some text and code, 링크의 뜻. item one item two");
    }

    #[test]
    fn summary_is_first_paragraph() {
        let r = render("## Title\n\nFirst *para*\ngraph.\n\nSecond.", None);
        assert_eq!(r.summary, "First para graph.");
    }
}
