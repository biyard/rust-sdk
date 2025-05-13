use dioxus::dioxus_core::Element;

#[cfg(feature = "server")]
pub use axum;
#[cfg(feature = "server")]
use axum::routing::Route;
#[cfg(feature = "server")]
use axum_core::{extract::Request, response::IntoResponse};
#[cfg(feature = "server")]
use std::convert::Infallible;
#[cfg(feature = "server")]
use tower_layer::Layer;
#[cfg(feature = "server")]
use tower_service::Service;

#[cfg(feature = "lambda")]
mod lambda;

pub mod prelude {
    pub use dioxus::prelude::*;
    pub use dioxus_fullstack::prelude::*;
}

#[doc = include_str!("../docs/launch.md")]
pub fn launch(_app: fn() -> Element) {
    #[cfg(feature = "web")]
    dioxus::launch(_app);

    #[cfg(feature = "mobile")]
    dioxus::launch(_app);

    #[cfg(feature = "server")]
    {
        use axum::routing::*;
        use dioxus_fullstack::prelude::*;

        struct TryIntoResult(Result<ServeConfig, dioxus_fullstack::UnableToLoadIndex>);

        impl TryInto<ServeConfig> for TryIntoResult {
            type Error = dioxus_fullstack::UnableToLoadIndex;

            fn try_into(self) -> Result<ServeConfig, Self::Error> {
                self.0
            }
        }

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let app = Router::new().serve_dioxus_application(
                    TryIntoResult(ServeConfigBuilder::default().build()),
                    _app,
                );

                #[cfg(not(feature = "lambda"))]
                {
                    let address = dioxus_cli_config::fullstack_address_or_localhost();
                    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

                    axum::serve(listener, app.into_make_service())
                        .await
                        .unwrap();
                }

                #[cfg(feature = "lambda")]
                {
                    use self::lambda::LambdaAdapter;
                    use tower_http::compression::CompressionLayer;
                    let app = app.layer(CompressionLayer::new());

                    tracing::info!("Running in lambda mode");
                    // lambda_http::run(app).await.unwrap();
                    lambda_runtime::run(LambdaAdapter::from(app)).await.unwrap();
                }
            });
    };
}

#[cfg(feature = "server")]
pub async fn launch_with_layers<L>(app: fn() -> Element, layers: Vec<L>)
where
    L: Layer<Route> + Clone + Send + 'static,
    L::Service: Service<Request> + Clone + Send + 'static,
    <L::Service as Service<Request>>::Response: IntoResponse + 'static,
    <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
    <L::Service as Service<Request>>::Future: Send + 'static,
{
    #[cfg(feature = "web")]
    dioxus::launch(app);

    #[cfg(feature = "mobile")]
    dioxus::launch(app);

    #[cfg(feature = "server")]
    {
        use axum::routing::*;
        use dioxus_fullstack::prelude::*;

        struct TryIntoResult(Result<ServeConfig, dioxus_fullstack::UnableToLoadIndex>);

        impl TryInto<ServeConfig> for TryIntoResult {
            type Error = dioxus_fullstack::UnableToLoadIndex;

            fn try_into(self) -> Result<ServeConfig, Self::Error> {
                self.0
            }
        }

        let mut app = Router::new()
            .serve_dioxus_application(TryIntoResult(ServeConfigBuilder::default().build()), app);

        for layer in layers {
            app = app.layer(layer);
        }

        #[cfg(not(feature = "lambda"))]
        {
            let address = dioxus_cli_config::fullstack_address_or_localhost();
            let listener = tokio::net::TcpListener::bind(address).await.unwrap();

            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        }

        #[cfg(feature = "lambda")]
        {
            use self::lambda::LambdaAdapter;

            tracing::info!("Running in lambda mode");
            lambda_runtime::run(LambdaAdapter::from(app)).await.unwrap();
        }
    };
}

#[cfg(feature = "server")]
use axum::routing::*;

#[cfg(feature = "server")]
pub fn new(app: fn() -> Element) -> axum::Router {
    use dioxus_fullstack::prelude::*;

    struct TryIntoResult(Result<ServeConfig, dioxus_fullstack::UnableToLoadIndex>);

    impl TryInto<ServeConfig> for TryIntoResult {
        type Error = dioxus_fullstack::UnableToLoadIndex;

        fn try_into(self) -> Result<ServeConfig, Self::Error> {
            self.0
        }
    }

    Router::new()
        .serve_dioxus_application(TryIntoResult(ServeConfigBuilder::default().build()), app)
}

#[cfg(feature = "server")]
pub fn serve(app: axum::Router) {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            #[cfg(not(feature = "lambda"))]
            {
                let address = dioxus_cli_config::fullstack_address_or_localhost();
                let listener = tokio::net::TcpListener::bind(address).await.unwrap();

                axum::serve(listener, app.into_make_service())
                    .await
                    .unwrap();
            }

            #[cfg(feature = "lambda")]
            {
                use self::lambda::LambdaAdapter;
                use tower_http::compression::CompressionLayer;
                let app = app.layer(CompressionLayer::new());

                tracing::info!("Running in lambda mode");
                // lambda_http::run(app).await.unwrap();
                lambda_runtime::run(LambdaAdapter::from(app.into_service()))
                    .await
                    .unwrap();
            }
        });
}
