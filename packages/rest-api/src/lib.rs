use std::{error::Error, sync::RwLock};

use reqwest::RequestBuilder;
use serde::Serialize;

pub mod signature;
pub use signature::Signature;

pub trait Signer {
    fn sign(&self, msg: &str) -> Result<Signature, Box<dyn Error>>;
    fn signer(&self) -> String;
}

static mut SIGNER: Option<RwLock<Box<dyn Signer>>> = None;
static mut MESSAGE: Option<String> = None;

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
    #[allow(static_mut_refs)]
    if let (Some(signer), Some(msg)) = unsafe { (&SIGNER, &MESSAGE) } {
        let signer = signer.read().unwrap();
        let address = signer.signer();
        if address.is_empty() {
            return req;
        }

        let timestamp = chrono::Utc::now().timestamp();
        let msg = format!("{}-{}", msg, timestamp);
        let signature = signer.sign(&msg);
        if signature.is_err() {
            return req;
        }

        let signature = signature.unwrap();
        req.header("Authorization", format!("UserSig {timestamp}:{signature}"))
    } else {
        req
    }
}

pub fn add_header_from_request(
    mut req: RequestBuilder,
    header_titles: Vec<&str>,
    header_values: Vec<&str>,
) -> RequestBuilder {
    if header_titles.len() != header_values.len() {
        return req;
    } else {
        for (title, value) in header_titles.iter().zip(header_values.iter()) {
            req = req.header(*title, *value);
        }
        req
    }
}

pub async fn get<T, E>(url: &str) -> Result<T, E>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned + From<reqwest::Error>,
{
    let client = reqwest::Client::builder().build()?;

    let req = client.get(url);

    let req = sign_request(req);
    let res = req.send().await?;

    if res.status().is_success() {
        Ok(res.json().await?)
    } else {
        Err(res.json().await?)
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

    let req = sign_request(req);
    let res = req.send().await?;

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
    E: serde::de::DeserializeOwned + From<reqwest::Error>,
{
    let client = reqwest::Client::builder().build()?;

    let req = client.post(url).json(&body);

    let req = sign_request(req);

    let res = req.send().await?;

    if res.status().is_success() {
        Ok(res.json().await?)
    } else {
        Err(res.json().await?)
    }
}
