//! Minimal dev server: serves `public/`, rebuilds when sources change and
//! tells open pages to reload (they poll `/__livereload`).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use walkdir::WalkDir;

use crate::site::{self, BuildOptions};

const WATCHED: &[&str] = &["content", "templates", "static", "i18n", "config.toml"];

pub fn run(root: PathBuf, out: PathBuf, drafts: bool, host: &str, port: u16) -> Result<()> {
    let base_url = format!(
        "http://{}:{port}",
        if host == "0.0.0.0" { "127.0.0.1" } else { host }
    );
    let opts = BuildOptions {
        root: root.clone(),
        out: out.clone(),
        drafts,
        base_url: Some(base_url.clone()),
        dev: true,
    };
    let version = Arc::new(AtomicU64::new(1));

    rebuild(&opts);
    {
        let version = version.clone();
        thread::spawn(move || {
            let mut last = latest_mtime(&root);
            loop {
                thread::sleep(Duration::from_millis(400));
                let now = latest_mtime(&root);
                if now != last {
                    last = now;
                    if rebuild(&opts) {
                        version.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        });
    }

    let listener = TcpListener::bind((host, port))?;
    println!("▶ Serving on {base_url}  (Ctrl+C to stop)");
    for stream in listener.incoming().flatten() {
        let out = out.clone();
        let version = version.clone();
        thread::spawn(move || {
            let _ = handle(stream, &out, &version);
        });
    }
    Ok(())
}

fn rebuild(opts: &BuildOptions) -> bool {
    let start = Instant::now();
    match site::build(opts) {
        Ok(s) => {
            println!("✔ Rebuilt {} files in {:.0?}", s.pages, start.elapsed());
            true
        }
        Err(e) => {
            eprintln!("✘ Build failed: {e:#}");
            false
        }
    }
}

fn latest_mtime(root: &Path) -> Option<SystemTime> {
    WATCHED
        .iter()
        .flat_map(|p| {
            WalkDir::new(root.join(p))
                .into_iter()
                .filter_map(|e| e.ok())
        })
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .max()
}

fn handle(mut stream: TcpStream, out: &Path, version: &AtomicU64) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let raw_path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let path = raw_path.split(['?', '#']).next().unwrap_or("/");

    if path == "/__livereload" {
        return respond(
            &mut stream,
            "200 OK",
            "text/plain",
            version.load(Ordering::SeqCst).to_string().as_bytes(),
        );
    }

    let decoded = percent_decode(path);
    if decoded.split('/').any(|seg| seg == "..") {
        return respond(&mut stream, "400 Bad Request", "text/plain", b"bad path");
    }
    let mut file = out.join(decoded.trim_start_matches('/'));
    if file.is_dir() {
        if !path.ends_with('/') {
            let loc = format!(
                "HTTP/1.1 301 Moved Permanently\r\nLocation: {path}/\r\nContent-Length: 0\r\n\r\n"
            );
            return stream.write_all(loc.as_bytes());
        }
        file = file.join("index.html");
    }
    match std::fs::read(&file) {
        Ok(body) => respond(&mut stream, "200 OK", mime(&file), &body),
        Err(_) => {
            let body =
                std::fs::read(out.join("404.html")).unwrap_or_else(|_| b"Not found".to_vec());
            respond(
                &mut stream,
                "404 Not Found",
                "text/html; charset=utf-8",
                &body,
            )
        }
    }
}

fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)
}

fn mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if let Some(v) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
