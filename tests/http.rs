//! Integration tests: exercise the HTTP surface from the crate root (static files, paths).

use website::routes;

#[tokio::test]
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
}

#[tokio::test]
async fn get_pelorus_same_as_root() {
    let filter = routes();
    let a = warp::test::request().path("/").reply(&filter).await;
    let b = warp::test::request().path("/pelorus").reply(&filter).await;
    assert_eq!(a.body(), b.body());
}

#[tokio::test]
async fn get_unknown_path_is_404() {
    let res = warp::test::request()
        .method("GET")
        .path("/no-such-page")
        .reply(&routes())
        .await;
    assert_eq!(res.status(), 404);
}

#[tokio::test]
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
