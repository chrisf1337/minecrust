use ::freetype::freetype as ft;
use failure::Fail;
use std::{fmt, os::raw::c_int};

macro_rules! ft_init {
    ( $t:expr , $e:expr ) => {{
        let err = $e;
        from_ft_err(err, $t)
    }};
}

mod library;

pub use self::library::Library;
pub use self::library::LoadFlags;

#[must_use]
pub type FtResult<T> = Result<T, FtError>;

#[derive(Debug, Fail)]
pub enum FtError {}

impl fmt::Display for FtError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl From<ft::FT_Error> for FtError {
    fn from(err: ft::FT_Error) -> FtError {
        match err {
            _ => unimplemented!(),
        }
    }
}

fn from_ft_err<T>(err: ft::FT_Error, t: T) -> FtResult<T> {
    if err == ft::FT_Err_Ok as c_int {
        Ok(t)
    } else {
        Err(err.into())
    }
}
