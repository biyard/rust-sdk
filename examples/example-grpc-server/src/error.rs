/// Service error; converted to `tonic::Status` by the `#[grpc]` wrapper.
#[derive(Debug)]
pub enum Error {
    Unauthenticated,
    NameRequired,
    CountdownTooLong { max: u32 },
}

impl From<Error> for by_grpc::Status {
    fn from(error: Error) -> Self {
        match error {
            Error::Unauthenticated => Self::unauthenticated("sign in required"),
            Error::NameRequired => Self::invalid_argument("name is required"),
            Error::CountdownTooLong { max } => {
                Self::invalid_argument(format!("countdown starts at most from {max}"))
            }
        }
    }
}
