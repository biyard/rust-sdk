use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Err(mismatches) = by_grpc::verify(&[example_grpc_server::DESCRIPTOR_SET]) {
        return Err(format!("proto/handler mismatch: {mismatches:?}").into());
    }
    let mut methods: Vec<_> = by_grpc::methods().collect();
    methods.sort_by_key(|m| m.path);
    for method in methods {
        println!(
            "registered {} ({}, {:?}) -> {}",
            method.path, method.version, method.kind, method.source
        );
    }

    let addr: SocketAddr = std::env::var("GRPC_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:50051".to_string())
        .parse()?;
    println!("listening on {addr}");
    by_grpc::serve(addr, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}
