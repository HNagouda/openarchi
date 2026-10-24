use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// A data service could not be reached or answered with an error.
    #[error("could not reach {service}: {detail}")]
    Network { service: String, detail: String },

    /// A data service answered, but not in the shape we expected.
    #[error("unexpected data from {service}: {detail}")]
    Data { service: String, detail: String },

    /// The caller asked for something that cannot work, like an area outside the Netherlands.
    #[error("{0}")]
    Invalid(String),

    #[error("file error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("could not read {path}: {detail}")]
    Format { path: String, detail: String },
}

impl Error {
    pub fn io(path: impl AsRef<std::path::Path>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }
}
