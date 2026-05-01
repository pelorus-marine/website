//! Integration tests: exercise the HTTP surface from the crate root (static files, paths).

use serial_test::serial;
use website::{routes, test_init_cache_sqlite_once};

#[tokio::test]
#[serial]
async fn get_root_is_html_with_tagline() {
    let res = warp::test::request()
        .method("GET")
        .path("/")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    let ctype = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert!(
        ctype.is_some_and(|c| c.starts_with("text/html")),
        "expected text/html, got {ctype:?}"
    );
    let body = std::str::from_utf8(res.body()).expect("utf-8 body");
    assert!(body.contains("By sailors, for sailors."));
    assert!(body.contains("pelorus-dial-root"));
}

#[tokio::test]
#[serial]
async fn get_pelorus_same_as_root() {
    let filter = routes();
    let a = warp::test::request().path("/").reply(&filter).await;
    let b = warp::test::request().path("/pelorus").reply(&filter).await;
    assert_eq!(a.body(), b.body());
}

#[tokio::test]
#[serial]
async fn get_unknown_path_is_404() {
    let res = warp::test::request()
        .method("GET")
        .path("/no-such-page")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 404);
    let ctype = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert!(
        ctype.is_some_and(|c| c.starts_with("text/html")),
        "expected text/html, got {ctype:?}"
    );
    let body = std::str::from_utf8(res.body()).expect("utf-8 body");
    assert!(body.contains("Oops"), "expected friendly 404 copy");
    assert!(body.contains("Back to home"));
}

#[tokio::test]
#[serial]
async fn static_favicon_png_served() {
    let res = warp::test::request()
        .method("GET")
        .path("/static/pelorus-favicon-32.png")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    assert!(!res.body().is_empty());
}

#[tokio::test]
#[serial]
async fn favicon_ico_alias_served() {
    let res = warp::test::request()
        .method("GET")
        .path("/favicon.ico")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    assert!(!res.body().is_empty());
}

#[tokio::test]
#[serial]
async fn static_pelorus_dial_js_served() {
    let res = warp::test::request()
        .method("GET")
        .path("/static/js/pelorus-dial.js")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    let body = std::str::from_utf8(res.body()).expect("utf-8 body");
    assert!(body.contains("mountPelorusDial"), "expected bundled dial script");
}

#[tokio::test]
#[serial]
async fn static_pelorus_dial_css_served() {
    let res = warp::test::request()
        .method("GET")
        .path("/static/css/pelorus-dial.css")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    let body = std::str::from_utf8(res.body()).expect("utf-8 body");
    assert!(body.contains("pelorus-dial-drift"), "expected dial animation css");
}

#[tokio::test]
#[serial]
async fn vendored_bootstrap_css_served() {
    let res = warp::test::request()
        .method("GET")
        .path("/static/vendor/bootstrap-5.3.3/bootstrap.min.css")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    let body = std::str::from_utf8(res.body()).expect("utf-8");
    assert!(body.contains(".container"), "expected bootstrap css");
    assert!(
        body.contains("Bootstrap"),
        "expected bootstrap banner comment"
    );
}

#[tokio::test]
#[serial]
async fn get_ecdis_returns_html_with_github() {
    test_init_cache_sqlite_once();
    let res = warp::test::request()
        .method("GET")
        .path("/ecdis")
        .reply(&routes())
        .await;
    assert!(
        res.status() == 200 || res.status() == 503,
        "unexpected {}",
        res.status()
    );
    let body = std::str::from_utf8(res.body()).expect("utf-8 body");
    assert!(body.contains("github.com/pelorus-marine/ecdis"));
    assert!(body.contains("raw.githubusercontent.com/pelorus-marine/ecdis"));
}

#[tokio::test]
#[serial]
async fn get_specifications_slash_returns_html_with_github() {
    test_init_cache_sqlite_once();
    let res = warp::test::request()
        .method("GET")
        .path("/specifications/")
        .reply(&routes())
        .await;
    assert!(
        res.status() == 200 || res.status() == 503,
        "unexpected {}",
        res.status()
    );
    let body = std::str::from_utf8(res.body()).expect("utf-8 body");
    assert!(body.contains("github.com/pelorus-marine/specifications"));
    assert!(body.contains("raw.githubusercontent.com/pelorus-marine/specifications"));
}

#[tokio::test]
#[serial]
async fn get_specifications_without_slash_redirects() {
    let res = warp::test::request()
        .method("GET")
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

#[tokio::test]
#[serial]
async fn bundled_display_font_woff2_served() {
    let res = warp::test::request()
        .method("GET")
        .path("/static/fonts/operation-napalm-regular.woff2")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 200);
    assert!(
        res.body().len() > 1000,
        "woff2 should be non-trivial size, got {} bytes",
        res.body().len()
    );
}
