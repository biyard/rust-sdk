use by_grpc::{CallParts, FromCall, Status};

use crate::Error;

/// Signed-in caller; the extractor form of the services' `auth::authenticated(&call)`.
#[derive(Debug, Clone)]
pub struct Authenticated {
    pub user_id: String,
}

impl FromCall for Authenticated {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        let user_id = parts
            .context()
            .caller
            .as_ref()
            .map(|caller| caller.user_id.trim())
            .unwrap_or_default();
        if user_id.is_empty() {
            return Err(Error::Unauthenticated.into());
        }
        Ok(Self {
            user_id: user_id.to_string(),
        })
    }
}

/// Workspace the BFF resolved for this call; the extractor form of `auth::visited_workspace(&call)`.
#[derive(Debug, Clone)]
pub struct VisitedWorkspace {
    pub workspace_id: String,
}

impl FromCall for VisitedWorkspace {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        parts
            .context()
            .workspace
            .as_ref()
            .map(|workspace| workspace.id.trim())
            .filter(|id| !id.is_empty())
            .map(|id| Self {
                workspace_id: id.to_string(),
            })
            .ok_or_else(|| Status::invalid_argument("this method requires a workspace scope"))
    }
}

/// Session cookie the BFF forwarded from the browser, if any.
#[derive(Debug, Clone)]
pub struct SessionCookie(pub Option<String>);

/// Name of the session cookie `SignIn` sets and `SignOut` clears.
pub const SESSION_COOKIE: &str = "sid";

impl FromCall for SessionCookie {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(Self(parts.context().cookies.get(SESSION_COOKIE).cloned()))
    }
}
