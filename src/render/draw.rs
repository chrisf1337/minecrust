use gl;
use gl::types::*;
use std;
use std::collections::HashSet;
use types::*;

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
}

impl ArrayBuffer {
    pub fn new(buf: Vec<f32>) -> ArrayBuffer {
        ArrayBuffer { id: 0, buf }
    }

    pub fn gl_init(&mut self) {
        unsafe {
            gl::CreateBuffers(1, &mut self.id as *mut u32);
        }
    }

    pub fn gl_bind(&self) {
        unsafe {
            gl::NamedBufferStorage(
                self.id,
                (self.buf.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                self.buf.as_ptr() as *const GLvoid,
                gl::MAP_READ_BIT,
            );
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn extend(&mut self, other: &Vec<f32>) {
        self.buf.extend(other)
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

    pub fn add_buffer(&mut self, buffer: ArrayBuffer) {
        self.total_buffer_len = buffer.len() as u32;
        self.buffer = buffer;
    }

    pub fn gl_init(&mut self) {
        unsafe {
            gl::CreateVertexArrays(1, &mut self.id as *mut u32);
        }
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

    pub fn gl_bind_all_buffers(&mut self) {
        self.buffer.gl_init();
        self.buffer.gl_bind();
        unsafe {
            gl::VertexArrayVertexBuffer(self.id, 0, self.buffer.id, 0, self.stride as i32);
        }
    }

    pub fn gl_draw(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
            gl::DrawArrays(gl::TRIANGLES, 0, self.total_buffer_len as i32);
        }
    }
}
