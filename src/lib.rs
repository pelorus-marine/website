//! Pelorus Marine website: warp routes and HTML rendering.

use std::convert::Infallible;

use askama::Template;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

pub const COPYRIGHT_START_YEAR: i32 = 2026;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    year: i32,
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundTemplate {
    year: i32,
}

/// Renders the home page HTML from the Askama template.
pub fn render_index_html() -> Result<String, askama::Error> {
    IndexTemplate {
        year: COPYRIGHT_START_YEAR,
    }
    .render()
}

fn render_not_found_html() -> Result<String, askama::Error> {
    NotFoundTemplate {
        year: COPYRIGHT_START_YEAR,
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

    let (code, message) = if err.find::<TemplateError>().is_some() {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Template render error"),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Internal Server Error"),
        )
    };
    Ok(warp::reply::with_status(warp::reply::html(message), code))
}

/// Full site filter: `/`, `/pelorus`, `/static/*`, `/favicon.ico`, plus rejection recovery.
pub fn routes() -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + 'static
{
    let index = warp::get()
        .and(warp::path::end())
        .and_then(|| async { render_index() });

    let pelorus = warp::get()
        .and(warp::path("pelorus"))
        .and(warp::path::end())
        .and_then(|| async { render_index() });

    let static_files = warp::path("static").and(warp::fs::dir("static"));

    let favicon = warp::path("favicon.ico").and(warp::fs::file("static/pelorus-favicon-32.png"));

    index
        .or(pelorus)
        .or(static_files)
        .or(favicon)
        .recover(handle_rejection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_html_contains_tagline_title_and_copyright_year() {
        let html = render_index_html().expect("template should render");
        assert!(html.contains("By sailors, for sailors."));
        assert!(html.contains("<title>Pelorus</title>"));
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
        assert!(body.contains("Oups"));
    }
}
