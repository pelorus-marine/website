//! SQLite cache for GitHub-sourced `ARCHITECTURE.md` (ephemeral; not in git).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::github::fetch_architecture_markdown;

/// Seconds a cached markdown body is treated as fresh before re-fetch is attempted.
pub(crate) const CACHE_TTL_SECS: i64 = 86_400;

const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS architecture_cache (
    slot TEXT PRIMARY KEY NOT NULL,
    source_url TEXT NOT NULL,
    markdown TEXT NOT NULL,
    fetched_at_unix INTEGER NOT NULL
);";

static CACHE_DB: std::sync::OnceLock<Arc<Mutex<Connection>>> = std::sync::OnceLock::new();

/// At most one in-flight background revalidation per architecture slot (avoid stampedes).
static REFRESH_IN_PROGRESS: [AtomicBool; 3] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

fn refresh_slot_index(slot: ArchitectureDocSlot) -> usize {
    match slot {
        ArchitectureDocSlot::Specifications => 0,
        ArchitectureDocSlot::Ecdis => 1,
        ArchitectureDocSlot::Platform => 2,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArchitectureDocSlot {
    Specifications,
    Ecdis,
    Platform,
}

impl ArchitectureDocSlot {
    pub(crate) fn key(self) -> &'static str {
        match self {
            ArchitectureDocSlot::Specifications => "specifications",
            ArchitectureDocSlot::Ecdis => "ecdis",
            ArchitectureDocSlot::Platform => "platform",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ResolveMarkdownError {
    /// No cached row and upstream fetch failed — caller should return 503.
    NoCacheAndUpstreamFailed,
}

pub(crate) fn default_cache_sqlite_path() -> std::path::PathBuf {
    std::env::temp_dir().join("pelorus-website-architecture-cache.sqlite")
}

pub fn bootstrap_cache_db_path() {
    if std::env::var_os("PELOURS_CACHE_DB_PATH").is_some() {
        return;
    }
    let path = default_cache_sqlite_path();
    let value = path.to_str().expect("temp_dir path must be valid UTF-8");
    // SAFETY: called synchronously from `main` before any Tokio worker threads are spawned.
    unsafe {
        std::env::set_var("PELOURS_CACHE_DB_PATH", value);
    }
}

pub fn init_cache_db(path: &Path) -> Result<(), rusqlite::Error> {
    if CACHE_DB.get().is_some() {
        return Ok(());
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;\nPRAGMA synchronous=NORMAL;")?;
    conn.execute_batch(SCHEMA)?;
    let arc = Arc::new(Mutex::new(conn));
    match CACHE_DB.set(arc) {
        Ok(()) => Ok(()),
        Err(_) => Ok(()),
    }
}

fn conn() -> &'static Arc<Mutex<Connection>> {
    CACHE_DB
        .get()
        .expect("init_cache_db must run before serving architecture pages")
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn is_fresh(fetched_at_unix: i64) -> bool {
    now_unix_secs().saturating_sub(fetched_at_unix) < CACHE_TTL_SECS
}

async fn read_row(
    slot: ArchitectureDocSlot,
) -> Result<Option<(String, String, i64)>, rusqlite::Error> {
    let key = slot.key().to_string();
    let db = conn().clone();
    tokio::task::spawn_blocking(move || {
        let guard = db.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT source_url, markdown, fetched_at_unix FROM architecture_cache WHERE slot = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        if let Some(r) = rows.next()? {
            let source_url: String = r.get(0)?;
            let markdown: String = r.get(1)?;
            let fetched_at: i64 = r.get(2)?;
            Ok(Some((source_url, markdown, fetched_at)))
        } else {
            Ok(None)
        }
    })
    .await
    .expect("spawn_blocking join")
}

async fn upsert_row(
    slot: ArchitectureDocSlot,
    source_url: &str,
    markdown: &str,
) -> Result<(), rusqlite::Error> {
    let key = slot.key().to_string();
    let source_url = source_url.to_string();
    let markdown = markdown.to_string();
    let now = now_unix_secs();
    let db = conn().clone();
    tokio::task::spawn_blocking(move || {
        let guard = db.lock().expect("sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO architecture_cache (slot, source_url, markdown, fetched_at_unix)
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(slot) DO UPDATE SET
                 source_url = excluded.source_url,
                 markdown = excluded.markdown,
                 fetched_at_unix = excluded.fetched_at_unix"#,
            rusqlite::params![key, source_url, markdown, now],
        )?;
        Ok::<_, rusqlite::Error>(())
    })
    .await
    .expect("spawn_blocking join")
}

pub(crate) async fn resolve_architecture_markdown(
    slot: ArchitectureDocSlot,
    source_url: &str,
) -> Result<String, ResolveMarkdownError> {
    let cached = read_row(slot)
        .await
        .map_err(|_| ResolveMarkdownError::NoCacheAndUpstreamFailed)?;

    match cached {
        None => match fetch_architecture_markdown(source_url).await {
            Ok(md) => {
                let _ = upsert_row(slot, source_url, &md).await;
                Ok(md)
            }
            Err(_) => Err(ResolveMarkdownError::NoCacheAndUpstreamFailed),
        },
        Some((_url, md, fetched_at)) if is_fresh(fetched_at) => Ok(md),
        Some((_url, md_stale, _fetched_at)) => {
            let ix = refresh_slot_index(slot);
            if REFRESH_IN_PROGRESS[ix]
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                let url = source_url.to_string();
                tokio::spawn(async move {
                    struct RefreshGuard(usize);
                    impl Drop for RefreshGuard {
                        fn drop(&mut self) {
                            REFRESH_IN_PROGRESS[self.0].store(false, Ordering::Release);
                        }
                    }

                    let _guard = RefreshGuard(ix);
                    if let Ok(md) = fetch_architecture_markdown(&url).await {
                        let _ = upsert_row(slot, &url, &md).await;
                    }
                });
            }
            Ok(md_stale)
        }
    }
}

/// Clears cached architecture markdown rows (for tests).
#[doc(hidden)]
pub fn test_clear_architecture_cache_rows() {
    let Some(db) = CACHE_DB.get() else {
        return;
    };
    let guard = db.lock().expect("sqlite mutex poisoned");
    let _ = guard.execute("DELETE FROM architecture_cache", []);
    REFRESH_IN_PROGRESS[0].store(false, Ordering::Release);
    REFRESH_IN_PROGRESS[1].store(false, Ordering::Release);
    REFRESH_IN_PROGRESS[2].store(false, Ordering::Release);
}

/// Opens a shared ephemeral SQLite file once for tests (must run before `routes()` architecture handlers).
#[doc(hidden)]
pub fn test_init_cache_sqlite_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let path = std::env::temp_dir().join(format!(
            "pelorus-website-test-architecture-cache-{}.sqlite",
            std::process::id()
        ));
        init_cache_db(&path).expect("sqlite init");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn default_sqlite_cache_file_name() {
        let p = default_cache_sqlite_path();
        assert!(
            p.to_string_lossy()
                .contains("pelorus-website-architecture-cache.sqlite")
        );
    }

    #[test]
    fn architecture_slot_keys() {
        assert_eq!(ArchitectureDocSlot::Specifications.key(), "specifications");
        assert_eq!(ArchitectureDocSlot::Ecdis.key(), "ecdis");
        assert_eq!(ArchitectureDocSlot::Platform.key(), "platform");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[serial]
    async fn resolve_cold_miss_then_cached_from_wiremock() {
        test_init_cache_sqlite_once();
        test_clear_architecture_cache_rows();

        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Wire"))
            .mount(&srv)
            .await;

        let url = format!("{}/ARCHITECTURE.md", srv.uri());
        let md = resolve_architecture_markdown(ArchitectureDocSlot::Specifications, &url)
            .await
            .unwrap();
        assert_eq!(md, "# Wire");

        let md2 = resolve_architecture_markdown(ArchitectureDocSlot::Specifications, &url)
            .await
            .unwrap();
        assert_eq!(md2, "# Wire");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[serial]
    async fn resolve_upstream_error_without_row() {
        test_init_cache_sqlite_once();
        test_clear_architecture_cache_rows();

        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(502))
            .mount(&srv)
            .await;

        let url = format!("{}/ARCHITECTURE.md", srv.uri());
        assert_eq!(
            resolve_architecture_markdown(ArchitectureDocSlot::Ecdis, &url).await,
            Err(ResolveMarkdownError::NoCacheAndUpstreamFailed)
        );
    }
}
