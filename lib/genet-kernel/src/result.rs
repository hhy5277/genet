use std::{error::Error, result};

pub type Result<T> = result::Result<T, Box<Error + Send>>;
