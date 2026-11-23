use by_grpc::{Reply, grpc};

use crate::Error;
use crate::auth::{Authenticated, SESSION_COOKIE, SessionCookie};
use crate::proto::v1::{SignOutReply, SignOutRequest};

/// `biyard.greeter.v1.Greeter/SignOut`: reads the forwarded session cookie and asks the BFF
/// to clear it.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn sign_out(
    _caller: Authenticated,
    session: SessionCookie,
    _req: SignOutRequest,
) -> Result<Reply<SignOutReply>, Error> {
    Ok(Reply::new(SignOutReply {
        had_session: session.0.is_some(),
    })
    .clear_cookie(SESSION_COOKIE))
}
