//! Pelorus Marine website: warp routes and HTML rendering.

mod architecture_cache;
mod github;
mod markdown_github;

use std::convert::Infallible;

use architecture_cache::{ArchitectureDocSlot, ResolveMarkdownError};
use askama::Template;
use warp::http::StatusCode;
use warp::path::FullPath;
use warp::{Filter, Rejection, Reply};

pub use architecture_cache::{bootstrap_cache_db_path, init_cache_db};
#[doc(hidden)]
pub use architecture_cache::{test_clear_architecture_cache_rows, test_init_cache_sqlite_once};

/// Resolve listen address from **`PORT`** (default **8080**). Binds IPv4 **`0.0.0.0`**.
#[must_use]
pub fn listen_socket_addr_from_env() -> std::net::SocketAddr {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
}

pub const COPYRIGHT_START_YEAR: i32 = 2026;

const SPECS_GITHUB: &str = "https://github.com/pelorus-marine/specifications";
const ECDIS_GITHUB: &str = "https://github.com/pelorus-marine/ecdis";
const PLATFORM_GITHUB: &str = "https://github.com/pelorus-marine/platform";

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    year: i32,
    specifications_nav_class: &'static str,
    ecdis_nav_class: &'static str,
    platform_nav_class: &'static str,
}

#[derive(Template)]
#[template(path = "error/404.html")]
struct NotFoundTemplate {
    year: i32,
    specifications_nav_class: &'static str,
    ecdis_nav_class: &'static str,
    platform_nav_class: &'static str,
}

#[derive(Template)]
#[template(path = "doc_gateway.html")]
struct DocGatewayTemplate {
    year: i32,
    page_title: &'static str,
    github_url: &'static str,
    architecture_md_url: String,
    specifications_nav_class: &'static str,
    ecdis_nav_class: &'static str,
    platform_nav_class: &'static str,
    body_html: String,
}

#[derive(Template)]
#[template(path = "error/503.html")]
struct TemporarilyUnavailableTemplate {
    year: i32,
    page_title: &'static str,
    github_url: &'static str,
    architecture_md_url: String,
    specifications_nav_class: &'static str,
    ecdis_nav_class: &'static str,
    platform_nav_class: &'static str,
}

#[derive(Template)]
#[template(path = "error/500.html")]
struct InternalErrorTemplate {
    year: i32,
    specifications_nav_class: &'static str,
    ecdis_nav_class: &'static str,
    platform_nav_class: &'static str,
}

fn github_ref_or_default() -> String {
    std::env::var("PELOURS_GITHUB_REF").unwrap_or_else(|_| "main".to_string())
}

fn specifications_architecture_md_url() -> String {
    std::env::var("PELOURS_SPECIFICATIONS_ARCHITECTURE_URL").unwrap_or_else(|_| {
        let github_ref = github_ref_or_default();
        format!("https://raw.githubusercontent.com/pelorus-marine/specifications/{github_ref}/ARCHITECTURE.md")
    })
}

fn ecdis_architecture_md_url() -> String {
    std::env::var("PELOURS_ECDIS_ARCHITECTURE_URL").unwrap_or_else(|_| {
        let github_ref = github_ref_or_default();
        format!(
            "https://raw.githubusercontent.com/pelorus-marine/ecdis/{github_ref}/ARCHITECTURE.md"
        )
    })
}

fn platform_architecture_md_url() -> String {
    std::env::var("PELOURS_PLATFORM_ARCHITECTURE_URL").unwrap_or_else(|_| {
        let github_ref = github_ref_or_default();
        format!(
            "https://raw.githubusercontent.com/pelorus-marine/platform/{github_ref}/ARCHITECTURE.md"
        )
    })
}

/// Renders the home page HTML from the Askama template.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_index_html() -> Result<String, askama::Error> {
    IndexTemplate {
        year: COPYRIGHT_START_YEAR,
        specifications_nav_class: "",
        ecdis_nav_class: "",
        platform_nav_class: "",
    }
    .render()
}

fn render_not_found_html() -> Result<String, askama::Error> {
    NotFoundTemplate {
        year: COPYRIGHT_START_YEAR,
        specifications_nav_class: "",
        ecdis_nav_class: "",
        platform_nav_class: "",
    }
    .render()
}

fn render_internal_server_error_html() -> Result<String, askama::Error> {
    InternalErrorTemplate {
        year: COPYRIGHT_START_YEAR,
        specifications_nav_class: "",
        ecdis_nav_class: "",
        platform_nav_class: "",
    }
    .render()
}

fn render_index() -> Result<impl Reply, Rejection> {
    match render_index_html() {
        Ok(body) => Ok(warp::reply::html(body)),
        Err(_) => Err(warp::reject::custom(TemplateError)),
    }
}

#[derive(Debug)]
struct TemplateError;
impl warp::reject::Reject for TemplateError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    if err.is_not_found() {
        let reply = match render_not_found_html() {
            Ok(body) => warp::reply::with_status(warp::reply::html(body), StatusCode::NOT_FOUND),
            Err(_) => warp::reply::with_status(
                warp::reply::html(String::from("<p>Not found</p>")),
                StatusCode::NOT_FOUND,
            ),
        };
        return Ok(reply);
    }

    let body = match render_internal_server_error_html() {
        Ok(html) => html,
        Err(_) => String::from("<!DOCTYPE html><title>Error</title><p>Internal Server Error</p>"),
    };
    Ok(warp::reply::with_status(
        warp::reply::html(body),
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

async fn render_architecture_gateway(
    slot: ArchitectureDocSlot,
    page_title: &'static str,
    github_url: &'static str,
    architecture_md_url: String,
    specifications_nav_class: &'static str,
    ecdis_nav_class: &'static str,
    platform_nav_class: &'static str,
) -> Result<impl Reply, Rejection> {
    match architecture_cache::resolve_architecture_markdown(slot, &architecture_md_url).await {
        Ok(md) => {
            let repo = match slot {
                ArchitectureDocSlot::Specifications => {
                    markdown_github::ArchitectureGithubRepo::Specifications
                }
                ArchitectureDocSlot::Ecdis => markdown_github::ArchitectureGithubRepo::Ecdis,
                ArchitectureDocSlot::Platform => markdown_github::ArchitectureGithubRepo::Platform,
            };
            let body_html =
                markdown_github::architecture_markdown_to_html(&md, repo, &github_ref_or_default());
            let tpl = DocGatewayTemplate {
                year: COPYRIGHT_START_YEAR,
                page_title,
                github_url,
                architecture_md_url,
                specifications_nav_class,
                ecdis_nav_class,
                platform_nav_class,
                body_html,
            };
            match tpl.render() {
                Ok(html) => Ok(warp::reply::with_status(
                    warp::reply::html(html),
                    StatusCode::OK,
                )),
                Err(_) => Err(warp::reject::custom(TemplateError)),
            }
        }
        Err(ResolveMarkdownError::NoCacheAndUpstreamFailed) => {
            let tpl = TemporarilyUnavailableTemplate {
                year: COPYRIGHT_START_YEAR,
                page_title,
                github_url,
                architecture_md_url,
                specifications_nav_class,
                ecdis_nav_class,
                platform_nav_class,
            };
            match tpl.render() {
                Ok(html) => Ok(warp::reply::with_status(
                    warp::reply::html(html),
                    StatusCode::SERVICE_UNAVAILABLE,
                )),
                Err(_) => Err(warp::reject::custom(TemplateError)),
            }
        }
    }
}

fn redirect_to_specifications_slash() -> impl Reply {
    warp::redirect::redirect(warp::http::Uri::from_static("/specifications/"))
}

/// Full site filter: `/`, `/pelorus`, `/ecdis`, `/platform`, `/specifications/` ( `/specifications` redirects ),
/// `/static/*`, `/favicon.ico`, plus rejection recovery.
///
/// Architecture pages resolve **`ARCHITECTURE.md`** via `SQLite` cache + GitHub raw URLs; stale cache is served
/// instantly while a single background task per slot revalidates when the TTL has passed.
pub fn routes() -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + 'static
{
    let index = warp::get()
        .and(warp::path::end())
        .and_then(|| async { render_index() });

    let pelorus = warp::get()
        .and(warp::path("pelorus"))
        .and(warp::path::end())
        .and_then(|| async { render_index() });

    let specifications_exact_redirect =
        warp::get()
            .and(warp::path::full())
            .and_then(|full: FullPath| async move {
                if full.as_str() == "/specifications" {
                    Ok(redirect_to_specifications_slash())
                } else {
                    Err(warp::reject::not_found())
                }
            });

    let specifications_doc =
        warp::get()
            .and(warp::path::full())
            .and_then(|full: FullPath| async move {
                if full.as_str() == "/specifications/" {
                    let url = specifications_architecture_md_url();
                    render_architecture_gateway(
                        ArchitectureDocSlot::Specifications,
                        "Specifications",
                        SPECS_GITHUB,
                        url,
                        "active",
                        "",
                        "",
                    )
                    .await
                } else {
                    Err(warp::reject::not_found())
                }
            });

    let ecdis_doc = warp::get()
        .and(warp::path("ecdis"))
        .and(warp::path::end())
        .and_then(|| async {
            let url = ecdis_architecture_md_url();
            render_architecture_gateway(
                ArchitectureDocSlot::Ecdis,
                "ECDIS",
                ECDIS_GITHUB,
                url,
                "",
                "active",
                "",
            )
            .await
        });

    let platform_doc = warp::get()
        .and(warp::path("platform"))
        .and(warp::path::end())
        .and_then(|| async {
            let url = platform_architecture_md_url();
            render_architecture_gateway(
                ArchitectureDocSlot::Platform,
                "Platform",
                PLATFORM_GITHUB,
                url,
                "",
                "",
                "active",
            )
            .await
        });

    let static_files = warp::path("static").and(warp::fs::dir("static"));

    let favicon = warp::path("favicon.ico").and(warp::fs::file("static/pelorus-favicon-32.png"));

    index
        .or(pelorus)
        .or(specifications_exact_redirect)
        .or(specifications_doc)
        .or(ecdis_doc)
        .or(platform_doc)
        .or(static_files)
        .or(favicon)
        .recover(handle_rejection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn ecdis_upstream_unreachable_without_cache_returns_503() {
        test_init_cache_sqlite_once();
        test_clear_architecture_cache_rows();
        temp_env::with_vars(
            vec![(
                "PELOURS_ECDIS_ARCHITECTURE_URL",
                Some("http://127.0.0.1:1/architecture.md"),
            )],
            || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        let res = warp::test::request().path("/ecdis").reply(&routes()).await;
                        assert_eq!(res.status(), 503);
                        let body = std::str::from_utf8(res.body()).expect("utf-8 body");
                        assert!(body.contains("Temporarily unavailable"));
                    });
            },
        );
    }

    #[test]
    fn markdown_to_html_renders_table() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let html = markdown_github::architecture_markdown_to_html(
            md,
            markdown_github::ArchitectureGithubRepo::Specifications,
            "main",
        );
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn rendered_html_contains_tagline_title_and_copyright_year() {
        let html = render_index_html().expect("template should render");
        assert!(html.contains("By sailors, for sailors."));
        assert!(html.contains("<title>Pelorus</title>"));
        assert!(html.contains("pelorus-dial-root"));
        assert!(html.contains("/static/js/pelorus-dial.js"));
        assert!(html.contains("/static/css/pelorus-dial.css"));
        assert!(html.contains("href=\"/platform\""));
        assert!(html.contains(&format!("&copy; {COPYRIGHT_START_YEAR} Pelorus Marine")));
    }

    #[test]
    fn rendered_html_contains_license_footer_text() {
        let html = render_index_html().expect("template should render");
        assert!(html.contains("Apache 2.0"));
        assert!(html.contains("proprietary"));
    }

    #[tokio::test]
    async fn routes_root_returns_200() {
        let res = warp::test::request()
            .method("GET")
            .path("/")
            .reply(&routes())
            .await;
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn routes_pelorus_matches_root_body() {
        let filter = routes();
        let root = warp::test::request()
            .method("GET")
            .path("/")
            .reply(&filter)
            .await;
        let pelorus = warp::test::request()
            .method("GET")
            .path("/pelorus")
            .reply(&filter)
            .await;
        assert_eq!(root.status(), 200);
        assert_eq!(pelorus.status(), 200);
        assert_eq!(root.body(), pelorus.body());
    }

    #[tokio::test]
    async fn routes_unknown_returns_404_html() {
        let res = warp::test::request()
            .method("GET")
            .path("/missing-page")
            .reply(&routes())
            .await;
        assert_eq!(res.status(), 404);
        let body = std::str::from_utf8(res.body()).expect("utf-8");
        assert!(body.contains("Oops"));
    }

    #[test]
    fn internal_error_template_renders_html() {
        let html = render_internal_server_error_html().expect("500 template");
        assert!(html.contains("Something went wrong"));
        assert!(html.contains("Oops"));
        assert!(html.contains("<title>Something went wrong — Pelorus</title>"));
    }

    #[tokio::test]
    #[serial]
    async fn routes_ecdis_returns_html_and_github_link() {
        test_init_cache_sqlite_once();
        let res = warp::test::request().path("/ecdis").reply(&routes()).await;
        assert!(
            res.status() == 200 || res.status() == 503,
            "unexpected status {}",
            res.status()
        );
        let body = std::str::from_utf8(res.body()).expect("utf-8");
        assert!(body.contains("github.com/pelorus-marine/ecdis"));
        assert!(body.contains("raw.githubusercontent.com/pelorus-marine/ecdis"));
    }

    #[tokio::test]
    #[serial]
    async fn routes_specifications_slash_returns_html_and_github_link() {
        test_init_cache_sqlite_once();
        let res = warp::test::request()
            .path("/specifications/")
            .reply(&routes())
            .await;
        assert!(
            res.status() == 200 || res.status() == 503,
            "unexpected status {}",
            res.status()
        );
        let body = std::str::from_utf8(res.body()).expect("utf-8");
        assert!(body.contains("github.com/pelorus-marine/specifications"));
        assert!(body.contains("raw.githubusercontent.com/pelorus-marine/specifications"));
    }

    #[tokio::test]
    #[serial]
    async fn routes_platform_returns_html_and_github_link() {
        test_init_cache_sqlite_once();
        let res = warp::test::request()
            .path("/platform")
            .reply(&routes())
            .await;
        assert!(
            res.status() == 200 || res.status() == 503,
            "unexpected status {}",
            res.status()
        );
        let body = std::str::from_utf8(res.body()).expect("utf-8");
        assert!(body.contains("github.com/pelorus-marine/platform"));
        assert!(body.contains("raw.githubusercontent.com/pelorus-marine/platform"));
    }

    #[tokio::test]
    async fn routes_specifications_without_slash_redirects() {
        let res = warp::test::request()
            .path("/specifications")
            .reply(&routes())
            .await;
        assert_eq!(res.status(), 301);
        let loc = res
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .expect("location header");
        assert_eq!(loc, "/specifications/");
    }
}
