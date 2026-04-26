use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
    println!("Pelorus website listening on http://{}", addr);
    warp::serve(website::routes()).run(addr).await;
}
