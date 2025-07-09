use std::error::Error;
pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;
