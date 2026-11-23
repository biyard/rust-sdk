use by_grpc::grpc;

use crate::Error;
use crate::auth::{Authenticated, VisitedWorkspace};
use crate::proto::v1::{WhoAmIReply, WhoAmIRequest};

/// `biyard.greeter.v1.Greeter/WhoAmI`: echoes the call context; requires a signed-in caller.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn who_am_i(
    caller: Authenticated,
    workspace: Option<VisitedWorkspace>,
    _req: WhoAmIRequest,
) -> Result<WhoAmIReply, Error> {
    Ok(WhoAmIReply {
        user_id: caller.user_id,
        workspace_id: workspace.map(|w| w.workspace_id),
    })
}
