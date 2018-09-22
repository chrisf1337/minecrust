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

use failure::{err_msg, Error};
use gl::types::*;
use glutin::{dpi::*, Event, GlContext, GlRequest, VirtualKeyCode, WindowEvent};
use minecrust::camera::Camera;
use minecrust::event_handlers::on_device_event;
use minecrust::geometry::{rectangle::Rectangle, square::Square, unitcube::UnitCube};
use minecrust::{debug, render, types::*, utils};
use na::{Isometry, Perspective3, Rotation3, Translation3, Unit};
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

fn main() -> Result<(), Error> {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_resizable(false)
        .with_dimensions(LogicalSize::new(1024., 768.));
    let screen_width = 1024;
    let screen_height = 768;
    let aspect_ratio = screen_width as f32 / screen_height as f32;
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
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    debug::enable_debug_output();

    let vert_shader = render::Shader::from_vert_source("src/shaders/triangle.vert")?;
    let frag_shader = render::Shader::from_frag_source("src/shaders/triangle.frag")?;
    let mut shader_program = render::Program::from_shaders(&[vert_shader, frag_shader])?;

    let crosshair_vshader = render::Shader::from_vert_source("src/shaders/crosshair.vert")?;
    let crosshair_fshader = render::Shader::from_frag_source("src/shaders/crosshair.frag")?;
    let mut crosshair_shader_program =
        render::Program::from_shaders(&[crosshair_vshader, crosshair_fshader])?;

    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }

    let cube1 = UnitCube::new(1.0);
    let mut cube2 = UnitCube::new(1.0);
    cube2.transform(&Translation3::from_vector(Vector3f::new(2.0, 0.0, -2.0)).to_homogeneous());
    let square = Square::new_with_transform(
        100.0,
        &Translation3::from_vector(Vector3f::new(0.0, -1.0, 0.0)).to_homogeneous(),
    );
    let mut vertices = square.vtx_data(&Matrix4f::identity());
    vertices.extend(cube1.vtx_data());
    vertices.extend(cube2.vtx_data());

    let cobblestone_path = "assets/cobblestone-border-arrow.png";
    let cobblestone_img = image::open(cobblestone_path)?.flipv().to_rgba();
    let cobblestone_img_data: Vec<u8> = cobblestone_img.clone().into_vec();

    let crosshair_path = "assets/crosshair.png";
    let crosshair_img = image::open(crosshair_path)?.flipv().to_rgba();
    let crosshair_img_data: Vec<u8> = crosshair_img.clone().into_vec();
    let crosshair_rect_height = 0.1;
    let crosshair_rect_width = crosshair_rect_height / aspect_ratio;
    let crosshair_rect = Rectangle::new_with_transform(
        crosshair_rect_width,
        crosshair_rect_height,
        &Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2).to_homogeneous(),
    );
    let crosshair_rect_vertices = crosshair_rect.vtx_data(&Matrix4f::identity());

    let mut vaos: [GLuint; 2] = [0, 0];
    let mut vbos: [GLuint; 2] = [0, 0];
    let mut textures: [GLuint; 2] = [0, 0];
    unsafe {
        gl::GenVertexArrays(2, vaos.as_mut_ptr());
        gl::GenBuffers(2, vbos.as_mut_ptr());
        gl::GenTextures(2, textures.as_mut_ptr());
    }
    let cobblestone_texture = textures[0];
    let crosshair_texture = textures[1];

    unsafe {
        gl::BindVertexArray(vaos[0]);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbos[0]);
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

        gl::BindVertexArray(vaos[1]);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbos[1]);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (crosshair_rect_vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
            crosshair_rect_vertices.as_ptr() as *const GLvoid,
            gl::STATIC_DRAW,
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

        gl::BindTexture(gl::TEXTURE_2D, cobblestone_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            cobblestone_img.width() as i32,
            cobblestone_img.height() as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            cobblestone_img_data.as_ptr() as *const GLvoid,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::BindTexture(gl::TEXTURE_2D, crosshair_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            crosshair_img.width() as i32,
            crosshair_img.height() as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            crosshair_img_data.as_ptr() as *const GLvoid,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    let mut camera = Camera::new_with_target(Point3f::new(3.0, 0.0, -3.0), Point3f::origin());
    let projection = Perspective3::new(
        screen_width as f32 / screen_height as f32,
        f32::to_radians(45.),
        0.1,
        100.,
    ).to_homogeneous();

    shader_program.set_used();
    shader_program.set_int("tex", 0); // gl::TEXTURE0
    crosshair_shader_program.set_used();
    crosshair_shader_program.set_int("tex", 0);

    let mut running = true;
    let mut focused = true;
    let mut pressed_keys = HashSet::new();
    let mut last_frame_time = SystemTime::now();
    while running {
        let mut mouse_delta = (0.0f64, 0.0f64);
        if focused {
            gl_window.grab_cursor(true).map_err(err_msg)?;
            gl_window.hide_cursor(true);
        }
        let current_frame_time = SystemTime::now();
        let delta_time = current_frame_time
            .duration_since(last_frame_time)?
            .as_nanos() as f32
            / 1.0e9;
        last_frame_time = current_frame_time;
        events_loop.poll_events(|event| match event {
            Event::DeviceEvent { event, .. } => {
                on_device_event(&event, &mut pressed_keys, &mut mouse_delta)
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::Focused(f) => focused = f,
                _ => (),
            },
            _ => (),
        });

        let d_yaw = mouse_delta.0 as f32 / 500.0;
        let d_pitch = mouse_delta.1 as f32 / 500.0;
        // Negate deltas because positive x is decreasing yaw, and positive y (origin for mouse
        // events is at upper left) is decreasing pitch.
        camera.rotate((-d_yaw, -d_pitch));
        if !utils::f32_almost_eq(d_yaw, 0.0) && !utils::f32_almost_eq(d_pitch, 0.0) {
            println!("{:#?}", camera);
            println!("{:?}", mouse_delta);
        }

        let camera_speed = 3.0 * delta_time;
        for keycode in &pressed_keys {
            match keycode {
                VirtualKeyCode::W => camera.pos += camera_speed * camera.direction().unwrap(),
                VirtualKeyCode::S => camera.pos -= camera_speed * camera.direction().unwrap(),
                VirtualKeyCode::A => {
                    let delta = camera_speed * (Vector3f::cross(&camera.direction(), &camera.up()));
                    camera.pos -= delta;
                }
                VirtualKeyCode::D => {
                    let delta = camera_speed * (Vector3f::cross(&camera.direction(), &camera.up()));
                    camera.pos += delta;
                }
                VirtualKeyCode::Escape => {
                    running = false;
                }
                _ => (),
            }
        }

        shader_program.set_used();
        shader_program.set_mat4f("view", &camera.to_matrix());
        shader_program.set_mat4f("projection", &projection);

        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, textures[0]);
            gl::BindVertexArray(vaos[0]);
            gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as i32);
        }

        crosshair_shader_program.set_used();
        unsafe {
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, textures[1]);
            gl::BindVertexArray(vaos[1]);
            gl::DrawArrays(gl::TRIANGLES, 0, crosshair_rect_vertices.len() as i32);
        }

        gl_window.swap_buffers().unwrap();
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    Ok(())
}
