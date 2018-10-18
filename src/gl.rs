pub mod shader;

use crate::types::*;
use gl;
use gl::types::*;
use std;
use std::collections::HashSet;

#[derive(Debug)]
pub struct AttributeFormat {
    pub location: u32,
    pub n_components: u32,
    pub normalized: bool,
}

#[derive(Default, Debug)]
pub struct ArrayBuffer {
    id: u32,
    buf: Vec<f32>,
    bound: bool,
}

impl ArrayBuffer {
    pub fn new(buf: Vec<f32>) -> ArrayBuffer {
        ArrayBuffer {
            id: 0,
            buf,
            bound: false,
        }
    }

    fn gl_init(&mut self) {
        unsafe {
            gl::CreateBuffers(1, &mut self.id as *mut u32);
        }
    }

    pub fn gl_bind(&mut self, usage: GlBufferUsage) {
        if !self.bound {
            unsafe {
                gl::NamedBufferData(
                    self.id,
                    (self.buf.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                    self.buf.as_ptr() as *const GLvoid,
                    usage.into(),
                );
            }
            self.bound = true;
        } else {
            unsafe {
                gl::NamedBufferSubData(
                    self.id,
                    0,
                    (self.buf.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                    self.buf.as_ptr() as *const GLvoid,
                );
            }
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn size(&self) -> usize {
        self.len() * std::mem::size_of::<f32>()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn extend(&mut self, other: &[f32]) {
        self.buf.extend(other);
    }

    pub fn set_buf(&mut self, buf: Vec<f32>) {
        self.buf = buf;
    }
}

#[derive(Default, Debug)]
pub struct VertexArrayObject {
    id: u32,
    attributes: Vec<AttributeFormat>,
    used_locations: HashSet<u32>,
    buffer: ArrayBuffer,
    stride: u32,
    total_buffer_len: u32,
}

impl VertexArrayObject {
    pub fn add_attribute(&mut self, format: AttributeFormat) {
        if self.used_locations.contains(&format.location) {
            panic!("location {} is already in use", format.location);
        }
        self.attributes.push(format);
    }

    pub fn gl_init(&mut self) {
        unsafe {
            gl::CreateVertexArrays(1, &mut self.id as *mut u32);
        }
        self.buffer.gl_init();
    }

    pub fn gl_setup_attributes(&mut self) {
        for attribute in &self.attributes {
            self.used_locations.insert(attribute.location);
            unsafe {
                gl::EnableVertexArrayAttrib(self.id, attribute.location);
                gl::VertexArrayAttribFormat(
                    self.id,
                    attribute.location,
                    attribute.n_components as i32,
                    gl::FLOAT,
                    gl::FALSE,
                    self.stride,
                );
                gl::VertexArrayAttribBinding(self.id, attribute.location, 0);
                self.stride += attribute.n_components * std::mem::size_of::<f32>() as u32;
            }
        }
    }

    pub fn gl_draw(&self) {
        unsafe {
            gl::VertexArrayVertexBuffer(self.id, 0, self.buffer.id, 0, self.stride as i32);
            gl::BindVertexArray(self.id);
            gl::DrawArrays(gl::TRIANGLES, 0, self.buffer.size() as i32);
        }
    }

    pub fn buffer_mut(&mut self) -> &mut ArrayBuffer {
        &mut self.buffer
    }
}
