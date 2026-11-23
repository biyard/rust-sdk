use by_grpc::{Reply, grpc};

use crate::Error;
use crate::auth::SESSION_COOKIE;
use crate::proto::v1::{SignInReply, SignInRequest};

/// `biyard.greeter.v1.Greeter/SignIn`: asks the BFF to set the session cookie.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn sign_in(req: SignInRequest) -> Result<Reply<SignInReply>, Error> {
    let user_id = req.user_id.trim().to_string();
    if user_id.is_empty() {
        return Err(Error::Unauthenticated);
    }
    let session = format!("session-{user_id}");
    Ok(Reply::new(SignInReply { user_id }).set_cookie(SESSION_COOKIE, session, 0))
}
