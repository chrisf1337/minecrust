#![feature(tool_lints, custom_attribute, duration_as_u128)]
#![allow(unknown_lints)]
#![warn(clippy::all)]
extern crate failure;
extern crate gl;
extern crate glutin;
extern crate image;
extern crate minecrust;
#[macro_use]
extern crate failure_derive;
extern crate nalgebra as na;

use gl::types::*;
use glutin::{
    dpi::*, DeviceEvent, Event, GlContext, GlRequest, KeyboardInput, VirtualKeyCode, WindowEvent,
};
use minecrust::event_handlers::on_device_event;
use minecrust::square::Square;
use minecrust::unitcube::UnitCube;
use minecrust::{debug, render, types::*, Camera};
use na::{Isometry, Perspective3, Rotation3, Translation3};
use std::collections::HashSet;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6, PI};
use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Fail, Debug)]
enum MainError {
    #[fail(display = "{}", _0)]
    NulError(#[cause] std::ffi::NulError),
    #[fail(display = "{}", _0)]
    Str(String),
    #[fail(display = "{}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "{}", _0)]
    ImageError(#[cause] image::ImageError),
    #[fail(display = "{}", _0)]
    SystemTimeError(#[cause] std::time::SystemTimeError),
}

impl From<std::ffi::NulError> for MainError {
    fn from(err: std::ffi::NulError) -> MainError {
        MainError::NulError(err)
    }
}

impl From<String> for MainError {
    fn from(err: String) -> MainError {
        MainError::Str(err)
    }
}

impl From<std::io::Error> for MainError {
    fn from(err: std::io::Error) -> MainError {
        MainError::Io(err)
    }
}

impl From<image::ImageError> for MainError {
    fn from(err: image::ImageError) -> MainError {
        MainError::ImageError(err)
    }
}

impl From<std::time::SystemTimeError> for MainError {
    fn from(err: std::time::SystemTimeError) -> MainError {
        MainError::SystemTimeError(err)
    }
}

fn main() -> Result<(), MainError> {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_resizable(false)
        .with_dimensions(LogicalSize::new(1024., 768.));
    let screen_width = 1024;
    let screen_height = 768;
    let context = glutin::ContextBuilder::new()
        .with_gl(GlRequest::Latest)
        .with_vsync(true);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        gl::load_with(|s| gl_window.get_proc_address(s) as *const _);
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Viewport(0, 0, screen_width, screen_height);
        gl::Enable(gl::DEPTH_TEST);
        // gl::DepthRange(1.0, 0.0);
    }

    debug::enable_debug_output();

    let vert_shader =
        render::Shader::from_vert_source(&CString::new(include_str!("../shaders/triangle.vert"))?)?;
    let frag_shader =
        render::Shader::from_frag_source(&CString::new(include_str!("../shaders/triangle.frag"))?)?;
    let mut shader_program = render::Program::from_shaders(&[vert_shader, frag_shader])?;

    // let mut unit_cube = UnitCube::new(1.0);
    // unit_cube.transform(&Translation3::from_vector(Vector3f::new(0.0, 0.0, 1.0)).to_homogeneous());
    // let mut vertices = unit_cube.vtx_data();

    // let mut unit_cube2 = UnitCube::new(1.0);
    // unit_cube2.transform(
    //     &Isometry::from_parts(
    //         Translation3::from_vector(Vector3f::new(1.0, 0.0, -1.0)),
    //         Rotation3::from_axis_angle(&Vector3f::z_axis(), PI / 12.0),
    //     ).to_homogeneous(),
    // );
    // vertices.extend_from_slice(&unit_cube2.vtx_data());

    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }

    // #[rustfmt_skip]
    // let square1 = Square::new_with_transform(
    //     1.0,
    //     // &Isometry::from_parts(
    //     //     Translation3::from_vector(Vector3f::new(0.0, 0.0, -1.0 / 2.0)),
    //     //     Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2),
    //     // ).to_homogeneous(),
    //     &Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2).to_homogeneous()
    // );
    // // let square2 = Square::new(1.0);
    // let mut vertices = square1.vtx_data(&Matrix4f::identity());
    // // vertices.extend_from_slice(&square2.vtx_data(&Matrix4f::identity()));
    // println!("{:#?}", vertices);

    let cube1 = UnitCube::new(1.0);
    let vertices = cube1.vtx_data();

    let cobblestone_path = "assets/cobblestone-border-arrow.png";
    let cobblestone_img = image::open(cobblestone_path)?.flipv().to_rgb();
    let cobblestone_img_data: Vec<u8> = cobblestone_img.clone().into_vec();

    let mut vao: GLuint = 0;
    let mut vbo: GLuint = 0;
    let mut texture: GLuint = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenTextures(1, &mut texture);

        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,                                            // target
            (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr, // size of data in bytes
            vertices.as_ptr() as *const GLvoid,                          // pointer to data
            gl::STATIC_DRAW,                                             // usage
        );

        gl::VertexAttribPointer(
            0,         // index of the generic vertex attribute ("layout (location = 0)")
            3,         // the number of components per generic vertex attribute
            gl::FLOAT, // data type
            gl::FALSE, // normalized (int-to-float conversion)
            (5 * std::mem::size_of::<f32>()) as GLint, // stride (byte offset between consecutive attributes)
            std::ptr::null(),                          // offset of the first component
        );
        gl::EnableVertexAttribArray(0); // this is "layout (location = 0)" in vertex shader

        gl::VertexAttribPointer(
            1,         // index of the generic vertex attribute ("layout (location = 1)")
            2,         // the number of components per generic vertex attribute
            gl::FLOAT, // data type
            gl::FALSE, // normalized (int-to-float conversion)
            (5 * std::mem::size_of::<f32>()) as GLint, // stride (byte offset between consecutive attributes)
            (3 * std::mem::size_of::<f32>()) as *const GLvoid, // offset of the first component
        );
        gl::EnableVertexAttribArray(1); // this is "layout (location = 1)" in vertex shader

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);

        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            cobblestone_img.width() as i32,
            cobblestone_img.height() as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            cobblestone_img_data.as_ptr() as *const GLvoid,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    let mut camera = Camera::new(
        Point3f::new(3.0, 3.0, 3.0),
        Point3f::origin(),
        Vector3f::y(),
    );
    let projection = Perspective3::new(
        screen_width as f32 / screen_height as f32,
        f32::to_radians(45.),
        0.1,
        100.,
    ).to_homogeneous();

    shader_program.set_used();
    shader_program.set_int("ourTexture", 0);

    let mut running = true;
    let mut focused = true;
    let mut pressed_keys = HashSet::new();
    let mut last_frame_time = SystemTime::now();
    while running {
        if focused {
            gl_window.grab_cursor(true)?;
            gl_window.hide_cursor(true);
        }
        let current_frame_time = SystemTime::now();
        let delta_time = current_frame_time
            .duration_since(last_frame_time)?
            .as_nanos() as f32
            / 1.0e9;
        last_frame_time = current_frame_time;
        events_loop.poll_events(|event| match event {
            Event::DeviceEvent { event, .. } => on_device_event(&event, &mut pressed_keys),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::Focused(f) => focused = f,
                _ => (),
            },
            _ => (),
        });

        let camera_speed = 5.0 * delta_time;
        for keycode in &pressed_keys {
            match keycode {
                VirtualKeyCode::W => camera.pos += camera_speed * camera.look_at_dir().as_ref(),
                VirtualKeyCode::S => camera.pos -= camera_speed * camera.look_at_dir().as_ref(),
                VirtualKeyCode::A => {
                    let delta = camera_speed * (Vector3f::cross(&camera.look_at_dir(), &camera.up));
                    camera.pos -= delta;
                    camera.target -= delta;
                }
                VirtualKeyCode::D => {
                    let delta = camera_speed * (Vector3f::cross(&camera.look_at_dir(), &camera.up));
                    camera.pos += delta;
                    camera.target += delta;
                }
                VirtualKeyCode::Escape => {
                    running = false;
                }
                _ => (),
            }
        }

        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);
        }

        shader_program.set_used();
        shader_program.set_mat4f("view", &camera.to_matrix());
        shader_program.set_mat4f("projection", &projection);

        unsafe {
            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as i32);
        }

        gl_window.swap_buffers().unwrap();
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    Ok(())
}
