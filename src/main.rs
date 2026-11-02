//! `blog` — a small multilingual static site generator for my tech blog.
//!
//! ```text
//! cargo run --release -- build            # → public/
//! cargo run --release -- serve            # http://127.0.0.1:1111 with live reload
//! cargo run --release -- new my-post ko   # scaffold content/posts/<today>-my-post/ko.md
//! ```

mod config;
mod content;
mod markdown;
mod serve;
mod site;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

const USAGE: &str = "\
Usage:
  blog build [--drafts] [--out <dir>] [--base-url <url>]
  blog serve [--drafts] [--port <port>] [--host <host>]
  blog new <slug> [lang]

Options:
  --drafts          Include posts with `draft = true`
  --out <dir>       Output directory (default: public)
  --base-url <url>  Override base_url from config.toml
  --port <port>     Dev server port (default: 1111)
  --host <host>     Dev server bind address (default: 127.0.0.1)";

struct Args {
    cmd: String,
    positional: Vec<String>,
    drafts: bool,
    out: PathBuf,
    base_url: Option<String>,
    port: u16,
    host: String,
}

fn parse_args() -> Result<Args> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "help".into());
    let mut a = Args {
        cmd,
        positional: Vec::new(),
        drafts: false,
        out: PathBuf::from("public"),
        base_url: None,
        port: 1111,
        host: "127.0.0.1".into(),
    };
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().with_context(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--drafts" => a.drafts = true,
            "--out" => a.out = PathBuf::from(value("--out")?),
            "--base-url" => a.base_url = Some(value("--base-url")?),
            "--port" => a.port = value("--port")?.parse().context("invalid --port")?,
            "--host" => a.host = value("--host")?,
            s if s.starts_with("--") => bail!("unknown option {s}\n\n{USAGE}"),
            _ => a.positional.push(arg),
        }
    }
    Ok(a)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let root = PathBuf::from(".");
    match args.cmd.as_str() {
        "build" => {
            let start = Instant::now();
            let stats = site::build(&site::BuildOptions {
                root,
                out: args.out.clone(),
                drafts: args.drafts,
                base_url: args.base_url,
                dev: false,
            })?;
            println!(
                "✔ Built {} files ({} posts) into {} in {:.0?}",
                stats.pages,
                stats.posts,
                args.out.display(),
                start.elapsed()
            );
        }
        "serve" => serve::run(root, args.out, args.drafts, &args.host, args.port)?,
        "new" => new_post(&args.positional)?,
        "help" | "--help" | "-h" => println!("{USAGE}"),
        other => bail!("unknown command `{other}`\n\n{USAGE}"),
    }
    Ok(())
}

fn new_post(args: &[String]) -> Result<()> {
    let Some(slug) = args.first() else {
        bail!("usage: blog new <slug> [lang]")
    };
    let cfg = config::Config::load(&PathBuf::from("."))?;
    let lang = args.get(1).cloned().unwrap_or(cfg.default_language.clone());
    if !cfg.languages.contains(&lang) {
        bail!("`{lang}` is not one of {:?}", cfg.languages);
    }
    let today = chrono::Local::now().date_naive();
    let slug = markdown::slugify(slug);
    let dir = PathBuf::from("content/posts").join(format!("{}-{slug}", today.format("%Y-%m-%d")));
    let file = dir.join(format!("{lang}.md"));
    if file.exists() {
        bail!("{} already exists", file.display());
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        &file,
        format!(
            "+++\ntitle = \"{slug}\"\ndescription = \"\"\ndate = {}\ntags = []\ndraft = true\n+++\n\nWrite here.\n",
            today.format("%Y-%m-%d")
        ),
    )?;
    println!("✔ Created {}", file.display());
    Ok(())
}
