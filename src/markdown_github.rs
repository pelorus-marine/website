//! Rewrite repo-relative Markdown links to GitHub blob URLs during HTML rendering.

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, html};

/// GitHub web UI base for [specifications](https://github.com/pelorus-marine/specifications).
const SPECS_REPO_WEB: &str = "https://github.com/pelorus-marine/specifications";
/// GitHub web UI base for [ecdis](https://github.com/pelorus-marine/ecdis).
const ECDIS_REPO_WEB: &str = "https://github.com/pelorus-marine/ecdis";

#[derive(Clone, Copy)]
pub(crate) enum ArchitectureGithubRepo {
    Specifications,
    Ecdis,
}

impl ArchitectureGithubRepo {
    pub(crate) fn web_base(self) -> &'static str {
        match self {
            ArchitectureGithubRepo::Specifications => SPECS_REPO_WEB,
            ArchitectureGithubRepo::Ecdis => ECDIS_REPO_WEB,
        }
    }
}

fn split_anchor(dest: &str) -> (&str, Option<&str>) {
    match dest.split_once('#') {
        Some((p, frag)) => (p, Some(frag)),
        None => (dest, None),
    }
}

fn skip_rewrite_dest(dest: &str) -> bool {
    let d = dest.trim();
    d.is_empty()
        || d.starts_with("http://")
        || d.starts_with("https://")
        || d.starts_with("mailto:")
        || d.starts_with("ftp:")
        || d.starts_with("data:")
        || d.starts_with("javascript:")
        || d.starts_with("//")
}

/// Normalize `./`, `../`, and repo-root paths (`/LICENSE`) against **`ARCHITECTURE.md`** living at the repo root.
fn normalize_repo_relative_path(path_part: &str) -> String {
    let mut trimmed = path_part.trim();
    trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    trimmed = trimmed.trim_start_matches('/');

    let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    let mut stack: Vec<&str> = Vec::new();
    for seg in segments {
        match seg {
            "." => {}
            ".." => {
                stack.pop();
            }
            _ => stack.push(seg),
        }
    }
    stack.join("/")
}

fn rewrite_markdown_link_dest(dest: &str, github_repo_web_base: &str, github_ref: &str) -> String {
    let dest = dest.trim();
    if skip_rewrite_dest(dest) {
        return dest.to_string();
    }

    let base = github_repo_web_base.trim_end_matches('/');
    let (path_part, fragment) = split_anchor(dest);

    if path_part.is_empty() {
        let tail = match fragment {
            Some(f) => format!("#{f}"),
            None => String::new(),
        };
        return format!("{base}/blob/{github_ref}/ARCHITECTURE.md{tail}");
    }

    let normalized = normalize_repo_relative_path(path_part);
    if normalized.is_empty() {
        let tail = match fragment {
            Some(f) => format!("#{f}"),
            None => String::new(),
        };
        return format!("{base}/blob/{github_ref}/ARCHITECTURE.md{tail}");
    }

    let mut out = format!("{base}/blob/{github_ref}/{normalized}");
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

pub(crate) fn architecture_markdown_to_html(
    md: &str,
    repo: ArchitectureGithubRepo,
    github_ref: &str,
) -> String {
    let github_repo_web_base = repo.web_base();
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(md, opts).map(|event| match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let new_dest =
                rewrite_markdown_link_dest(dest_url.as_ref(), github_repo_web_base, github_ref);
            Event::Start(Tag::Link {
                link_type,
                dest_url: CowStr::Boxed(new_dest.into_boxed_str()),
                title,
                id,
            })
        }
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let new_dest =
                rewrite_markdown_link_dest(dest_url.as_ref(), github_repo_web_base, github_ref);
            Event::Start(Tag::Image {
                link_type,
                dest_url: CowStr::Boxed(new_dest.into_boxed_str()),
                title,
                id,
            })
        }
        e => e,
    });

    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    html_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_slash_md_becomes_blob_url() {
        assert_eq!(
            rewrite_markdown_link_dest("./stream/01-overview.md", SPECS_REPO_WEB, "main"),
            "https://github.com/pelorus-marine/specifications/blob/main/stream/01-overview.md"
        );
    }

    #[test]
    fn hash_only_points_at_architecture_md() {
        assert_eq!(
            rewrite_markdown_link_dest("#lmde", SPECS_REPO_WEB, "main"),
            "https://github.com/pelorus-marine/specifications/blob/main/ARCHITECTURE.md#lmde"
        );
    }

    #[test]
    fn absolute_https_left_alone() {
        assert_eq!(
            rewrite_markdown_link_dest("https://example.com/foo", SPECS_REPO_WEB, "main"),
            "https://example.com/foo"
        );
    }

    #[test]
    fn architecture_html_contains_blob_url_for_relative_link() {
        let md = "[Stream](./stream/01-overview.md)";
        let html =
            architecture_markdown_to_html(md, ArchitectureGithubRepo::Specifications, "main");
        assert!(html.contains("pelorus-marine/specifications/blob/main/stream/01-overview.md"));
    }

    #[test]
    fn mailto_and_protocol_relative_skipped() {
        assert_eq!(
            rewrite_markdown_link_dest("mailto:a@b.c", SPECS_REPO_WEB, "main"),
            "mailto:a@b.c"
        );
        assert_eq!(
            rewrite_markdown_link_dest("//evil.example/foo", SPECS_REPO_WEB, "main"),
            "//evil.example/foo"
        );
    }

    #[test]
    fn parent_segments_normalized() {
        assert_eq!(
            rewrite_markdown_link_dest("../LICENSE", ECDIS_REPO_WEB, "feature/x"),
            "https://github.com/pelorus-marine/ecdis/blob/feature/x/LICENSE"
        );
    }

    #[test]
    fn image_relative_rewrites_to_blob() {
        let md = "![d](./diagram.png)";
        let html = architecture_markdown_to_html(md, ArchitectureGithubRepo::Ecdis, "main");
        assert!(html.contains("pelorus-marine/ecdis/blob/main/diagram.png"));
    }
}
