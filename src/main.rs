use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    website::bootstrap_cache_db_path();
    let db_path = env::var("PELOURS_CACHE_DB_PATH")?;
    website::init_cache_db(Path::new(&db_path))?;

    let addr = website::listen_socket_addr_from_env();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            println!("Pelorus website listening on http://{addr}");
            warp::serve(website::routes()).run(addr).await;
        });

    Ok(())
}
