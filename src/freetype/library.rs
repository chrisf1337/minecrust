use crate::freetype::{from_ft_err, FtResult};
use ::freetype::freetype as ft;
use bitflags::bitflags;
use std::{ffi::CString, os::raw::c_long, path::Path};

#[derive(Debug, Clone)]
pub struct Library {
    lib: ft::FT_Library,
}

#[derive(Debug, Clone)]
pub struct Face {
    face: ft::FT_Face,
    pub glyph: Option<Glyph>,
}

impl Library {
    pub fn new() -> FtResult<Library> {
        unsafe {
            let mut ft_lib = std::ptr::null_mut();
            let lib = ft_init!(ft_lib, ft::FT_Init_FreeType(&mut ft_lib))?;
            Ok(Library { lib })
        }
    }

    pub fn new_face<P: AsRef<Path>>(&self, filepath: P, face_index: c_long) -> FtResult<Face> {
        let mut face = std::ptr::null_mut();
        let path = CString::new(filepath.as_ref().to_str().unwrap()).unwrap();
        unsafe {
            let face = ft_init!(
                face,
                ft::FT_New_Face(self.lib, path.as_ptr(), face_index, &mut face)
            )?;
            Ok(Face { face, glyph: None })
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            ft::FT_Done_FreeType(self.lib);
        }
    }
}

bitflags! {
    pub struct LoadFlags: u32 {
        const Render = ft::FT_LOAD_RENDER;
    }
}

impl Face {
    pub fn set_pixel_sizes(&self, pixel_width: u32, pixel_height: u32) -> FtResult<()> {
        unsafe {
            from_ft_err(
                ft::FT_Set_Pixel_Sizes(self.face, pixel_width, pixel_height),
                (),
            )
        }
    }

    pub fn load_char(&mut self, ch: char, flags: LoadFlags) -> FtResult<()> {
        unsafe {
            from_ft_err(
                ft::FT_Load_Char(self.face, ch as ft::FT_ULong, flags.bits as i32),
                (),
            )?;
            self.glyph = Some(Glyph::new((*self.face).glyph));
            Ok(())
        }
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe {
            ft::FT_Done_Face(self.face);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GlyphMetrics {
    pub width: ft::FT_Pos,
    pub height: ft::FT_Pos,
    pub hori_bearing_x: ft::FT_Pos,
    pub hori_bearing_y: ft::FT_Pos,
    pub hori_advance: ft::FT_Pos,
    pub vert_bearing_x: ft::FT_Pos,
    pub vert_bearing_y: ft::FT_Pos,
    pub vert_advance: ft::FT_Pos,
}

impl From<ft::FT_Glyph_Metrics> for GlyphMetrics {
    fn from(metrics: ft::FT_Glyph_Metrics) -> GlyphMetrics {
        GlyphMetrics {
            width: metrics.width / 64,
            height: metrics.height / 64,
            hori_bearing_x: metrics.horiBearingX / 64,
            hori_bearing_y: metrics.horiBearingY / 64,
            hori_advance: metrics.horiAdvance / 64,
            vert_bearing_x: metrics.vertBearingX / 64,
            vert_bearing_y: metrics.vertBearingY / 64,
            vert_advance: metrics.vertAdvance / 64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Glyph {
    pub bitmap: Bitmap,
    pub metrics: GlyphMetrics,
}

impl Glyph {
    fn new(glyph: ft::FT_GlyphSlot) -> Glyph {
        unsafe {
            Glyph {
                bitmap: Bitmap::new((*glyph).bitmap),
                metrics: (*glyph).metrics.into(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bitmap {
    pub rows: u32,
    pub width: u32,
    pub pitch: i32,
    pub buffer: Vec<u8>,
}

impl Bitmap {
    fn new(bitmap: ft::FT_Bitmap) -> Bitmap {
        let buffer_len = bitmap.rows as usize * bitmap.pitch.abs() as usize;
        let mut buffer = vec![0; buffer_len];
        unsafe {
            std::ptr::copy_nonoverlapping(bitmap.buffer, buffer.as_mut_ptr(), buffer_len);
        }
        Bitmap {
            rows: bitmap.rows,
            width: bitmap.width,
            pitch: bitmap.pitch,
            buffer,
        }
    }
}
