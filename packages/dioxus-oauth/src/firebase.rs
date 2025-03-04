#[cfg(feature = "web")]
use dioxus::hooks::use_effect;
#[cfg(feature = "web")]
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::Promise;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = firebase, js_name = initializeApp)]
    pub fn initialize_app(config: &JsValue);

    #[wasm_bindgen(js_namespace = firebase, js_name = getAuth)]
    pub fn get_auth() -> GoogleAuth;

    #[wasm_bindgen(catch, js_namespace = firebase, js_name = signInWithPopup)]
    pub fn sign_in_with_popup(
        auth: &GoogleAuth,
        provider: &GoogleAuthProvider,
    ) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch, js_namespace = firebase, js_name = signOut)]
    pub async fn sign_out(auth: &GoogleAuth) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type UserCredentialImpl;

    #[wasm_bindgen(method, getter = user)]
    pub fn user(this: &UserCredentialImpl) -> GoogleUser;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type GoogleAuth;

    #[wasm_bindgen(method, getter = currentUser)]
    pub fn current_user(this: &GoogleAuth) -> Option<GoogleAuthUser>;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type GoogleAuthUser;

    #[wasm_bindgen(method, js_name = getIdToken)]
    pub async fn get_id_token(this: &GoogleAuthUser) -> JsValue;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type GoogleUser;

    #[wasm_bindgen(method, getter = displayName)]
    pub fn display_name(this: &GoogleUser) -> String;

    #[wasm_bindgen(method, getter = email)]
    pub fn email(this: &GoogleUser) -> String;

    #[wasm_bindgen(method, getter = photoURL)]
    pub fn photo_url(this: &GoogleUser) -> String;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type GoogleAuthProvider;

    #[wasm_bindgen(js_namespace = firebase, constructor)]
    pub fn new() -> GoogleAuthProvider;

    #[wasm_bindgen(method, js_name = addScope)]
    pub fn add_scope(this: &GoogleAuthProvider, scope: &str);

    #[wasm_bindgen(js_namespace = firebase, static_method_of = GoogleAuthProvider, js_name = credentialFromResult)]
    pub fn credential_from_result(result: &JsValue) -> Option<GoogleCredential>;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type GoogleCredential;

    #[wasm_bindgen(method, getter = accessToken)]
    pub fn access_token(this: &GoogleCredential) -> String;

    #[wasm_bindgen(method, getter = idToken)]
    pub fn id_token(this: &GoogleCredential) -> String;
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirebaseConfig {
    pub api_key: String,
    pub auth_domain: String,
    pub project_id: String,
    pub storage_bucket: String,
    pub messaging_sender_id: String,
    pub app_id: String,
    pub measurement_id: String,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Credential {
    pub id_token: String,
    pub access_token: String,
    pub display_name: String,
    pub email: String,
    pub photo_url: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FirebaseService {}

impl FirebaseService {
    pub fn new(
        api_key: String,
        auth_domain: String,
        project_id: String,
        storage_bucket: String,
        messaging_sender_id: String,
        app_id: String,
        measurement_id: String,
    ) -> Self {
        tracing::debug!("FirebaseService::init: {api_key}, {auth_domain}, {project_id}, {storage_bucket}, {messaging_sender_id}, {app_id}, {measurement_id}");
        #[cfg(feature = "web")]
        use_effect(move || {
            let config = FirebaseConfig {
                api_key: api_key.clone(),
                auth_domain: auth_domain.clone(),
                project_id: project_id.clone(),
                storage_bucket: storage_bucket.clone(),
                messaging_sender_id: messaging_sender_id.clone(),
                app_id: app_id.clone(),
                measurement_id: measurement_id.clone(),
            };

            let v = serde_wasm_bindgen::to_value(&config).unwrap_or_default();

            tracing::debug!("initialize firebase");
            initialize_app(&v);
        });

        Self {
            // config: use_signal(move || config),
        }
    }

    pub async fn sign_in_with_popup(&self, scopes: Vec<String>) -> Credential {
        let auth = get_auth();
        let provider = GoogleAuthProvider::new();
        for scope in scopes {
            provider.add_scope(&scope);
        }

        let cred: Credential = match sign_in_with_popup(&auth, &provider) {
            Ok(res) => {
                let res = JsFuture::from(res).await;
                if res.is_err() {
                    tracing::error!("{:?}", res);
                    return Credential::default();
                }
                let res = res.unwrap();
                let credential = GoogleAuthProvider::credential_from_result(&res);

                if credential.is_none() {
                    tracing::error!("empty credential has been returned");
                    return Credential::default();
                }

                let v = credential.unwrap();
                let access_token = v.access_token();
                let id_token = v.id_token();
                tracing::debug!("id_token: {id_token}, access_token: {access_token}");
                web_sys::console::log_1(&res);

                let cred: UserCredentialImpl = res.dyn_into().unwrap();
                let user = cred.user();

                Credential {
                    id_token: self.get_user_id_token().await.unwrap_or_default(),
                    access_token,
                    display_name: user.display_name(),
                    email: user.email(),
                    photo_url: user.photo_url(),
                }
            }
            e => {
                tracing::error!("FirebaseService::sign_in_with_popup: {:?}", e);
                Credential::default()
            }
        };

        cred
    }

    pub async fn get_user_id_token(&self) -> Option<String> {
        let auth = get_auth();
        let user = auth.current_user();
        if user.is_none() {
            tracing::error!("No user found");
            return None;
        }

        let user = user.unwrap();
        let id_token = user.get_id_token().await;
        let id_token = id_token.as_string().unwrap_or_default();
        tracing::debug!("id_token: {id_token}");

        Some(id_token)
    }
}
