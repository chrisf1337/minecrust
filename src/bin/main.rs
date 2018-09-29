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
extern crate specs;

use failure::{err_msg, Error};
use gl::types::*;
use glutin::{dpi::*, Event, GlContext, GlRequest, VirtualKeyCode, WindowEvent};
use minecrust::camera::Camera;
use minecrust::ecs::{PrimitiveGeometryComponent, TransformComponent};
use minecrust::event_handlers::on_device_event;
use minecrust::geometry::{rectangle::Rectangle, square::Square, unitcube::UnitCube};
use minecrust::render::shader::{Program, Shader};
use minecrust::render::state::{ArrayBuffer, AttributeFormat, VertexArrayObject};
use minecrust::{debug, types::*};
use na::{Perspective3, Rotation3, Translation3};
use specs::{Builder, World};
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;
use std::time::SystemTime;

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
        .with_multisampling(8)
        .with_vsync(true);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        gl::load_with(|s| gl_window.get_proc_address(s) as *const _);
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Viewport(0, 0, screen_width, screen_height);
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::Enable(gl::MULTISAMPLE);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    debug::enable_debug_output();

    let vert_shader = Shader::from_vert_source("src/shaders/triangle.vert")?;
    let frag_shader = Shader::from_frag_source("src/shaders/triangle.frag")?;
    let mut shader_program = Program::from_shaders(&[vert_shader, frag_shader])?;

    let crosshair_vshader = Shader::from_vert_source("src/shaders/crosshair.vert")?;
    let crosshair_fshader = Shader::from_frag_source("src/shaders/crosshair.frag")?;
    let mut crosshair_shader_program =
        Program::from_shaders(&[crosshair_vshader, crosshair_fshader])?;

    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }

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

    let selection_path = "assets/selection.png";
    let selection_img = image::open(selection_path)?.flipv().to_rgba();
    let selection_img_data: Vec<u8> = selection_img.clone().into_vec();

    let square = Square::new_with_transform(
        100.0,
        &Translation3::from_vector(Vector3f::new(0.0, -1.0, 0.0)).to_homogeneous(),
    );
    let cube1 = UnitCube::new(1.0);
    let cube2 = UnitCube::new(1.0);
    let cube3 = UnitCube::new(1.0);
    let cube1_vtxs = cube1.vtx_data(&Matrix4f::identity());
    let mut buffer = ArrayBuffer::new(
        cube2.vtx_data(&Translation3::from_vector(Vector3f::new(2.0, 0.0, -2.0)).to_homogeneous()),
    );
    buffer.extend(&cube1_vtxs);
    buffer.extend(&square.vtx_data(&Matrix4f::identity()));
    buffer.extend(
        &cube3
            .vtx_data(&Translation3::from_vector(Vector3f::new(-2.0, 1.0, -2.0)).to_homogeneous()),
    );
    let mut vao = VertexArrayObject::default();
    vao.add_attribute(AttributeFormat {
        location: 0,
        n_components: 3,
        normalized: false,
    });
    vao.add_attribute(AttributeFormat {
        location: 1,
        n_components: 2,
        normalized: false,
    });
    vao.add_buffer(buffer);
    vao.gl_init();
    vao.gl_setup_attributes();
    vao.gl_bind_all_buffers();

    let mut selection_vao = VertexArrayObject::default();
    let selection_buffer = ArrayBuffer::new(cube1_vtxs);
    selection_vao.add_attribute(AttributeFormat {
        location: 0,
        n_components: 3,
        normalized: false,
    });
    selection_vao.add_attribute(AttributeFormat {
        location: 1,
        n_components: 2,
        normalized: false,
    });
    selection_vao.add_buffer(selection_buffer);
    selection_vao.gl_init();
    selection_vao.gl_setup_attributes();
    selection_vao.gl_bind_all_buffers();

    let mut crosshair_vao = VertexArrayObject::default();
    let crosshair_buffer = ArrayBuffer::new(crosshair_rect.vtx_data(&Matrix4f::identity()));
    crosshair_vao.add_attribute(AttributeFormat {
        location: 0,
        n_components: 3,
        normalized: false,
    });
    crosshair_vao.add_attribute(AttributeFormat {
        location: 1,
        n_components: 2,
        normalized: false,
    });
    crosshair_vao.add_buffer(crosshair_buffer);
    crosshair_vao.gl_init();
    crosshair_vao.gl_setup_attributes();
    crosshair_vao.gl_bind_all_buffers();

    let mut world = World::new();
    world.register::<TransformComponent>();
    world.register::<PrimitiveGeometryComponent>();

    let cube = world
        .create_entity()
        .with(TransformComponent::new(Matrix4f::identity()))
        .with(PrimitiveGeometryComponent::new_unit_cube(UnitCube::new(
            1.0,
        )));

    let mut vaos: [GLuint; 3] = [0; 3];
    let mut vbos: [GLuint; 3] = [0; 3];
    let mut textures: [GLuint; 3] = [0; 3];
    unsafe {
        gl::GenVertexArrays(3, vaos.as_mut_ptr());
        gl::GenBuffers(3, vbos.as_mut_ptr());
        gl::GenTextures(3, textures.as_mut_ptr());
    }
    let cobblestone_texture = textures[0];
    let crosshair_texture = textures[1];
    let selection_texture = textures[2];

    unsafe {
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

        gl::BindTexture(gl::TEXTURE_2D, selection_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            selection_img.width() as i32,
            selection_img.height() as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            selection_img_data.as_ptr() as *const GLvoid,
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
    )
    .to_homogeneous();

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
        // if !utils::f32_almost_eq(d_yaw, 0.0) && !utils::f32_almost_eq(d_pitch, 0.0) {
        //     println!("{:#?}", camera);
        //     println!("{:?}", mouse_delta);
        // }

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

        unsafe {
            gl::DepthFunc(gl::LESS);
        }

        shader_program.set_used();
        shader_program.set_mat4f("view", &camera.to_matrix());
        shader_program.set_mat4f("projection", &projection);

        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, textures[0]);
        }
        vao.gl_draw();

        unsafe {
            gl::DepthFunc(gl::LEQUAL);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, textures[2]);
        }
        selection_vao.gl_draw();

        crosshair_shader_program.set_used();
        unsafe {
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, textures[1]);
        }
        crosshair_vao.gl_draw();

        gl_window.swap_buffers().unwrap();
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    Ok(())
}
