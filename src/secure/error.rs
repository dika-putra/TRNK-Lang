use std::fmt;

#[derive(Debug)]
pub enum Error {
    WrongCredentialsError,
    UnauthorizedError,
    InternalError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WrongCredentialsError => f.write_str("wrong credentials"),
            Error::UnauthorizedError => f.write_str("unauthorized"),
            Error::InternalError(m) => write!(f, "internal error: {}", m),
        }
    }
}

impl std::error::Error for Error {}

impl warp::reject::Reject for Error {}
