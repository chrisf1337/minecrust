#![feature(tool_lints, custom_attribute)]
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
use minecrust::ecs::{PrimitiveGeometryComponent, RenderSystem, TransformComponent};
use minecrust::event_handlers::on_device_event;
use minecrust::geometry::{rectangle::Rectangle, square::Square, unitcube::UnitCube};
use minecrust::render::shader::{Program, Shader};
use minecrust::render::state::{ArrayBuffer, AttributeFormat, RenderState, VertexArrayObject};
use minecrust::{debug, types::*};
use na::{Perspective3, Rotation3, Translation3};
use specs::{Builder, DispatcherBuilder, World};
use std::f32::consts::FRAC_PI_2;
use std::ops::DerefMut;
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

    let cube1 = UnitCube::new(1.0);
    let cube1_vtxs = cube1.vtx_data(&Matrix4f::identity());

    let mut selection_vao = VertexArrayObject::default();
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
    selection_vao.gl_init();
    selection_vao.gl_setup_attributes();
    selection_vao.buffer.set_buf(cube1_vtxs);
    selection_vao.buffer.gl_init();
    selection_vao.buffer.gl_bind();
    selection_vao.gl_set_binding();

    let mut crosshair_vao = VertexArrayObject::default();
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
    crosshair_vao.gl_init();
    crosshair_vao.gl_setup_attributes();
    {
        let crosshair_buffer = &mut crosshair_vao.buffer;
        crosshair_buffer.set_buf(crosshair_rect.vtx_data(&Matrix4f::identity()));
        crosshair_buffer.gl_init();
        crosshair_buffer.gl_bind();
    }
    crosshair_vao.gl_set_binding();

    let mut textures: [GLuint; 3] = [0; 3];
    unsafe {
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

    let camera = Camera::new_with_target(Point3f::new(3.0, 0.0, -3.0), Point3f::origin());
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
    let mut last_frame_time = SystemTime::now();

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
    vao.buffer.gl_init();
    vao.gl_init();
    vao.gl_setup_attributes();
    vao.gl_set_binding();

    let mut render_state = RenderState::new(
        camera,
        shader_program,
        crosshair_shader_program,
        cobblestone_texture,
        selection_texture,
        crosshair_texture,
        projection,
    );
    render_state.vao = vao;
    render_state.selection_vao = selection_vao;
    render_state.crosshair_vao = crosshair_vao;

    let mut world = World::new();
    world.register::<TransformComponent>();
    world.register::<PrimitiveGeometryComponent>();
    world.add_resource(render_state);
    let mut dispatcher = DispatcherBuilder::new()
        .with_thread_local(RenderSystem)
        .build();

    // square
    world
        .create_entity()
        .with(TransformComponent::new(
            Translation3::from_vector(Vector3f::new(0.0, -1.0, 0.0)).to_homogeneous(),
        ))
        .with(PrimitiveGeometryComponent::new_square(Square::new(100.0)))
        .build();
    // cube 1
    world
        .create_entity()
        .with(TransformComponent::new(Matrix4f::identity()))
        .with(PrimitiveGeometryComponent::new_unit_cube(UnitCube::new(
            1.0,
        )))
        .build();
    // cube 2
    world
        .create_entity()
        .with(TransformComponent::new(
            Translation3::from_vector(Vector3f::new(2.0, 0.0, -2.0)).to_homogeneous(),
        ))
        .with(PrimitiveGeometryComponent::new_unit_cube(UnitCube::new(
            1.0,
        )))
        .build();
    // cube 3
    world
        .create_entity()
        .with(TransformComponent::new(
            Translation3::from_vector(Vector3f::new(-2.0, 1.0, -2.0)).to_homogeneous(),
        ))
        .with(PrimitiveGeometryComponent::new_unit_cube(UnitCube::new(
            1.0,
        )))
        .build();

    while running {
        let current_frame_time = SystemTime::now();
        let frame_time_delta = current_frame_time.duration_since(last_frame_time)?;
        last_frame_time = current_frame_time;
        let mut mouse_delta = (0.0f64, 0.0f64);
        {
            let mut render_state = world.write_resource::<RenderState>();
            let render_state = render_state.deref_mut();
            let pressed_keys = &mut render_state.pressed_keys;

            if focused {
                gl_window.grab_cursor(true).map_err(err_msg)?;
                gl_window.hide_cursor(true);
            }
            events_loop.poll_events(|event| match event {
                Event::DeviceEvent { event, .. } => {
                    on_device_event(&event, pressed_keys, &mut mouse_delta);
                    if pressed_keys.contains(&VirtualKeyCode::Escape) && focused {
                        running = false;
                    }
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => running = false,
                    WindowEvent::Focused(f) => focused = f,
                    _ => (),
                },
                _ => (),
            });

            render_state.mouse_delta = mouse_delta;
            render_state.frame_time_delta = frame_time_delta;
        }
        dispatcher.dispatch(&world.res);
        world.maintain();

        gl_window.swap_buffers().unwrap();
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    Ok(())
}
