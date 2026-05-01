//! HTTPS fetch of markdown sources (GitHub raw).

use std::time::Duration;

use reqwest::Client;

pub(crate) const MAX_ARCHITECTURE_BYTES: usize = 2 * 1024 * 1024;

fn http_client() -> &'static Client {
    static CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(concat!(
                "PelorusWebsite/",
                env!("CARGO_PKG_VERSION"),
                " (+https://sevenseas.io/pelorus)",
            ))
            .build()
            .expect("reqwest Client builds")
    })
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FetchError {
    Request,
    UpstreamStatus,
    TooLarge,
    NotUtf8,
}

/// Validates HTTP status and body length/encoding after bytes are received (unit-testable).
pub(crate) fn validate_architecture_response(
    status: u16,
    bytes: &[u8],
) -> Result<String, FetchError> {
    if bytes.len() > MAX_ARCHITECTURE_BYTES {
        return Err(FetchError::TooLarge);
    }
    if !(200..300).contains(&status) {
        return Err(FetchError::UpstreamStatus);
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| FetchError::NotUtf8)
}

pub(crate) async fn fetch_architecture_markdown(url: &str) -> Result<String, FetchError> {
    let response = http_client()
        .get(url)
        .send()
        .await
        .map_err(|_| FetchError::Request)?;
    let status = response.status().as_u16();
    let bytes = response.bytes().await.map_err(|_| FetchError::Request)?;
    validate_architecture_response(status, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_success() {
        let s = validate_architecture_response(200, b"# Hello").unwrap();
        assert_eq!(s, "# Hello");
    }

    #[test]
    fn validate_non_success_status() {
        assert_eq!(
            validate_architecture_response(404, b"nope"),
            Err(FetchError::UpstreamStatus)
        );
        assert_eq!(
            validate_architecture_response(500, b""),
            Err(FetchError::UpstreamStatus)
        );
    }

    #[test]
    fn validate_too_large() {
        let blob = vec![b'a'; MAX_ARCHITECTURE_BYTES + 1];
        assert_eq!(
            validate_architecture_response(200, &blob),
            Err(FetchError::TooLarge)
        );
    }

    #[test]
    fn validate_boundary_ok_at_max_size() {
        let blob = vec![b'b'; MAX_ARCHITECTURE_BYTES];
        assert!(validate_architecture_response(200, &blob).is_ok());
    }

    #[test]
    fn validate_invalid_utf8() {
        assert_eq!(
            validate_architecture_response(200, &[0xff, 0xfe]),
            Err(FetchError::NotUtf8)
        );
    }

    #[tokio::test]
    async fn fetch_hits_wiremock_200() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ARCHITECTURE.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# From mock"))
            .mount(&srv)
            .await;

        let url = format!("{}/ARCHITECTURE.md", srv.uri());
        let body = fetch_architecture_markdown(&url).await.unwrap();
        assert_eq!(body, "# From mock");
    }

    #[tokio::test]
    async fn fetch_wiremock_404() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&srv)
            .await;

        assert_eq!(
            fetch_architecture_markdown(&format!("{}/x", srv.uri()))
                .await
                .unwrap_err(),
            FetchError::UpstreamStatus
        );
    }
}
