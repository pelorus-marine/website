//! Integration tests that touch process environment use **`temp-env`** scopes and **`serial_test::serial`**
//! so mutations never overlap across threads. The coverage CI job runs tests with **`--test-threads=1`**
//! as defense-in-depth.

use serial_test::serial;

#[test]
#[serial]
fn bootstrap_sets_default_sqlite_when_pe_missing() {
    temp_env::with_vars(vec![("PELOURS_CACHE_DB_PATH", None::<&str>)], || {
        website::bootstrap_cache_db_path();
        let path =
            std::env::var("PELOURS_CACHE_DB_PATH").expect("bootstrap sets PELOURS_CACHE_DB_PATH");
        assert!(
            path.contains("pelorus-website-architecture-cache.sqlite"),
            "{path}"
        );
    });
}

#[test]
#[serial]
fn bootstrap_respects_existing_pe() {
    temp_env::with_vars(
        vec![(
            "PELOURS_CACHE_DB_PATH",
            Some("/tmp/custom-pelorus-architecture.sqlite"),
        )],
        || {
            website::bootstrap_cache_db_path();
            assert_eq!(
                std::env::var("PELOURS_CACHE_DB_PATH").unwrap(),
                "/tmp/custom-pelorus-architecture.sqlite"
            );
        },
    );
}

#[test]
#[serial]
fn listen_port_defaults_to_8080() {
    temp_env::with_vars(vec![("PORT", None::<&str>)], || {
        assert_eq!(website::listen_socket_addr_from_env().port(), 8080);
    });
}

#[test]
#[serial]
fn listen_port_parses_env() {
    temp_env::with_vars(vec![("PORT", Some("9555"))], || {
        assert_eq!(website::listen_socket_addr_from_env().port(), 9555);
    });
}

#[test]
#[serial]
fn listen_invalid_port_falls_back_to_8080() {
    temp_env::with_vars(vec![("PORT", Some("not-a-port"))], || {
        assert_eq!(website::listen_socket_addr_from_env().port(), 8080);
    });
}
