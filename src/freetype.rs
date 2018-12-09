use ::freetype::freetype as ft;
use failure::Fail;
use std::fmt;

pub type FtResult<T> = Result<T, FtError>;

#[derive(Debug, Fail)]
pub enum FtError {}

impl fmt::Display for FtError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl From<::freetype::FT_Error> for FtError {
    fn from(err: ::freetype::FT_Error) -> FtError {
        match err {
            _ => unimplemented!(),
        }
    }
}
