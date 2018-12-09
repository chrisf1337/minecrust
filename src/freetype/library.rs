#[derive(Debug, Clone, Copy)]
pub struct Library {
    lib: freetype::FT_Library,
}

impl Library {
    fn new() -> FtResult<Library> {
        unsafe {
            let mut ft_lib = std::ptr::null_mut();
            ft::FT_Init_FreeType(&mut ft_lib)?;
        }
    }

    // fn new_face(&self) ->
}
