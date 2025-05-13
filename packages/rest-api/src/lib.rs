#![allow(static_mut_refs)]
use std::{collections::HashMap, error::Error, future::Future, pin::Pin, sync::RwLock};

use reqwest::RequestBuilder;
use serde::Serialize;

pub mod signature;
pub use signature::Signature;

pub trait Signer {
    fn sign(&self, msg: &str) -> Result<Signature, Box<dyn Error>>;
    fn signer(&self) -> String;
}

pub trait RequestHooker {
    fn before_request(&self, req: RequestBuilder) -> RequestBuilder;
}

pub trait ApiService {
    fn handle(
        &mut self,
        req: RequestBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<reqwest::Response, reqwest::Error>> + Send>>;
}

static mut SIGNER: Option<RwLock<Box<dyn Signer>>> = None;
static mut MESSAGE: Option<String> = None;
// FIXME: It causes dropping Signal of dioxus
// static mut HOOKS: RwLock<Vec<Box<dyn RequestHooker>>> = RwLock::new(Vec::new());
static mut HEADERS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);
static mut API_SERVICE: Option<Box<dyn ApiService>> = None;

pub fn set_api_service(service: Box<dyn ApiService>) {
    unsafe {
        API_SERVICE = Some(service);
    }
}
// pub fn add_hook<T: RequestHooker + 'static>(hook: T) {
//     unsafe {
//         HOOKS.write().unwrap().push(Box::new(hook));
//     }
// }

// pub fn run_hooks(req: RequestBuilder) -> RequestBuilder {
//     unsafe {
//         HOOKS
//             .read()
//             .unwrap()
//             .iter()
//             .fold(req, |req, hook| hook.before_request(req))
//     }
// }

pub fn get_authz_token() -> Option<String> {
    unsafe {
        let headers = HEADERS.read().unwrap();
        match headers.as_ref() {
            Some(headers) => headers
                .get("Authorization")
                .cloned()
                .unwrap_or_default()
                .split(" ")
                .last()
                .map(|s| s.to_string()),
            _ => None,
        }
    }
}

pub fn get_header(key: &str) -> Option<String> {
    unsafe {
        let headers = HEADERS.read().unwrap();
        match headers.as_ref() {
            Some(headers) => headers.get(key).cloned(),
            None => None,
        }
    }
}

pub fn add_header(key: String, value: String) {
    unsafe {
        let mut headers = HEADERS.write().unwrap();
        match headers.as_mut() {
            Some(headers) => {
                headers.insert(key, value);
            }
            None => {
                let mut new_headers = HashMap::new();
                new_headers.insert(key, value);
                *headers = Some(new_headers);
            }
        }
    }
}

pub fn remove_header(key: &str) {
    unsafe {
        let mut headers = HEADERS.write().unwrap();
        match headers.as_mut() {
            Some(headers) => {
                headers.remove(key);
            }
            None => {}
        }
    }
}

pub fn set_signer(signer: Box<dyn Signer>) {
    unsafe {
        SIGNER = Some(RwLock::new(signer));
    }
}

pub fn remove_signer() {
    unsafe {
        SIGNER = None;
    }
}

pub fn set_message(msg: String) {
    unsafe {
        MESSAGE = Some(msg);
    }
}

pub fn sign_request(req: RequestBuilder) -> RequestBuilder {
    if let (Some(signer), Some(msg)) = unsafe { (&SIGNER, &MESSAGE) } {
        let signer = signer.read().unwrap();
        let address = signer.signer();
        tracing::debug!("Signer address: {}", address);
        if address.is_empty() {
            return req;
        }

        let timestamp = chrono::Utc::now().timestamp();
        let msg = format!("{}-{}", msg, timestamp);
        tracing::debug!("Signing message: {}", msg);
        let signature = signer.sign(&msg);
        if signature.is_err() {
            return req;
        }

        let signature = signature.unwrap();
        req.header(
            reqwest::header::AUTHORIZATION,
            format!("UserSig {timestamp}:{signature}"),
        )
    } else {
        tracing::debug!("No signer found");
        req
    }
}

pub fn add_authorization(token: &str) {
    unsafe {
        let mut headers = HEADERS.write().unwrap();
        match headers.as_mut() {
            Some(headers) => {
                headers.insert("Authorization".to_string(), token.to_string());
            }
            None => {
                let mut new_headers = HashMap::new();
                new_headers.insert("Authorization".to_string(), token.to_string());
                *headers = Some(new_headers);
            }
        }
    }
}

pub fn extract_for_next_request(res: &reqwest::Response) {
    let headers = res.headers();
    if let Some(authz) = headers.get(reqwest::header::AUTHORIZATION) {
        let authz = authz.to_str().unwrap();
        add_authorization(authz);
    } else if let Some(authz) = headers.get("x-amzn-remapped-authorization") {
        let authz = authz.to_str().unwrap();
        add_authorization(authz);
    }
}

pub fn load_headers(mut req: RequestBuilder) -> RequestBuilder {
    unsafe {
        match HEADERS.read().unwrap().as_ref() {
            Some(ref headers) => {
                for (k, v) in headers.iter() {
                    req = req.header(k, v);
                }

                req
            }
            None => req,
        }
    }
}

pub async fn send(req: RequestBuilder) -> reqwest::Result<reqwest::Response> {
    // let req = run_hooks(req);
    let req = sign_request(req);
    let req = load_headers(req);

    let api_service = unsafe { API_SERVICE.as_mut() };
    let res = match api_service {
        Some(api_service) => api_service.handle(req).await,
        None => req.send().await,
    };

    if let Ok(res) = &res {
        extract_for_next_request(res);
    }

    res
}
pub async fn get<T, E>(url: &str) -> Result<T, E>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned + From<reqwest::Error> + From<gloo_net::Error>,
{
    #[cfg(feature = "server")]
    {
        let client = reqwest::Client::builder().build()?;
        #[allow(unused_mut)]
        let mut req = client.get(url);

        #[cfg(not(feature = "test"))]
        {
            use dioxus_fullstack::prelude::server_context;
            use reqwest::header::{HeaderMap, AUTHORIZATION, COOKIE};

            let ctx = server_context();

            let headers: HeaderMap = ctx.extract().await.unwrap();
            let auth_token_value = headers
                .get(COOKIE)
                .and_then(|cookie_header_value| cookie_header_value.to_str().ok())
                .and_then(|cookie_str| {
                    cookie_str.split(';').find_map(|cookie_pair| {
                        let mut parts = cookie_pair.trim().splitn(2, '=');
                        let key = parts.next()?;
                        let value = parts.next()?;
                        if key == "auth_token" {
                            Some(value.to_string())
                        } else {
                            None
                        }
                    })
                });
            if let Some(auth_token_value) = auth_token_value {
                tracing::debug!("auth_token_value: {}", auth_token_value);
                req = req.header(AUTHORIZATION, auth_token_value)
            };
        }

        let res = send(req).await?;

        if res.status().is_success() {
            return Ok(res.json().await?);
        } else {
            return Err(res.json().await?);
        }
    }

    #[allow(unused)]
    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::builder().build()?;
        #[allow(unused_mut)]
        let mut req = client.get(url);

        let res = send(req).await?;

        if res.status().is_success() {
            return Ok(res.json().await?);
        } else {
            return Err(res.json().await?);
        }
    }

    #[allow(unused)]
    #[cfg(feature = "web")]
    {
        let res = gloo_net::http::Request::get(url)
            .header("Content-Type", "application/json")
            .credentials(web_sys::RequestCredentials::Include)
            .send()
            .await?;
        let status_code = res.status();
        if status_code < 400 {
            Ok(res.json().await?)
        } else {
            Err(res.json().await?)
        }
    }
}

/// Performs an HTTP GET request.
///
/// # Arguments
///
/// * `url` - The URL to send the request to
/// * `query_params` - Query parameters for the URL. Pass `&None::<()>` to send request without query parameters
///
///
pub async fn get_with_query<T, E, P>(url: &str, query_params: &P) -> Result<T, E>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned + From<reqwest::Error>,
    P: serde::Serialize + ?Sized,
{
    let client = reqwest::Client::builder().build()?;

    let req = client.get(url).query(query_params);
    let res = send(req).await?;

    if res.status().is_success() {
        Ok(res.json().await?)
    } else {
        Err(res.json().await?)
    }
}
pub async fn post<R, T, E>(url: &str, body: R) -> Result<T, E>
where
    R: Serialize,
    T: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned + From<reqwest::Error> + From<gloo_net::Error>,
{
    #[cfg(any(feature = "server", not(target_arch = "wasm32")))]
    {
        let client = reqwest::Client::builder().build()?;

        let req = client.post(url).json(&body);
        let res = send(req).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(res.json().await?)
        }
    }

    #[cfg(all(not(feature = "server"), target_arch = "wasm32"))]
    {
        tracing::debug!("POST(Web) {}", url);
        let req = gloo_net::http::Request::post(url)
            .header("Content-Type", "application/json")
            .credentials(web_sys::RequestCredentials::Include)
            .json(&body)?;
        let res = req.send().await?;
        let status_code = res.status();
        if status_code < 400 {
            Ok(res.json().await?)
        } else {
            Err(res.json().await?)
        }
    }
}
