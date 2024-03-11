use snafu::{Location, Snafu};


#[derive(Snafu,Debug)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("shutdown channel is close"))]
    Shutdown {
        #[snafu(source)]
        error: std::io::Error,
    },

    #[snafu(display("Failed to send prometheus remote request"))]
    SendPromRemoteRequest {
        location: Location,
        #[snafu(source)]
        error: reqwest::Error,
    },
}
pub type Result<T> = std::result::Result<T, Error>;