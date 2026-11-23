use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;

use tonic::service::Routes;
use tonic::Status;

use crate::method::GrpcMethod;

/// Registry inconsistency found while building [`routes`].
#[derive(Debug)]
pub enum RegistryError {
    /// Two `#[grpc]` functions claim the same gRPC path.
    DuplicatePath {
        path: &'static str,
        first: &'static str,
        second: &'static str,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePath {
                path,
                first,
                second,
            } => write!(f, "{path} is registered by both {first} and {second}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Builds tonic routes from every registered [`GrpcMethod`]; unknown paths answer `UNIMPLEMENTED`.
pub fn routes() -> Result<Routes, RegistryError> {
    let mut seen: HashMap<&'static str, &'static GrpcMethod> = HashMap::new();
    let mut router = axum::Router::new();
    for method in crate::methods() {
        if let Some(first) = seen.insert(method.path, method) {
            return Err(RegistryError::DuplicatePath {
                path: method.path,
                first: first.source,
                second: method.source,
            });
        }
        let handler = method.handler;
        router = router.route_service(
            method.path,
            tower::service_fn(move |request: http::Request<axum::body::Body>| async move {
                Ok::<_, Infallible>(handler(request).await)
            }),
        );
    }
    Ok(Routes::from(router.fallback(unimplemented)))
}

/// Serves every registered method on `addr` until `shutdown` resolves.
pub async fn serve(
    addr: SocketAddr,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tonic::transport::Server::builder()
        .add_routes(routes()?)
        .serve_with_shutdown(addr, shutdown)
        .await?;
    Ok(())
}

async fn unimplemented() -> http::Response<tonic::body::Body> {
    let (parts, ()) = Status::unimplemented("").into_http::<()>().into_parts();
    http::Response::from_parts(parts, tonic::body::Body::empty())
}
