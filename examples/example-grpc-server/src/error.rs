/// Service error; converted to `tonic::Status` by the `#[grpc]` wrapper.
#[derive(Debug)]
pub enum Error {
    Unauthenticated,
    NameRequired,
}

impl From<Error> for tonic::Status {
    fn from(error: Error) -> Self {
        match error {
            Error::Unauthenticated => tonic::Status::unauthenticated("sign in required"),
            Error::NameRequired => tonic::Status::invalid_argument("name is required"),
        }
    }
}
