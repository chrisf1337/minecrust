use crate::{
    camera::{Camera, CameraAnimation},
    ecs::{
        BoundingBoxComponent, BoundingBoxComponentSystem, PrimitiveGeometryComponent, RenderSystem,
        TransformComponent,
    },
    event_handlers::on_device_event,
    geometry::{square::Square, unitcube::UnitCube, PrimitiveGeometry},
    na::Translation3,
    renderer::Renderer,
    types::prelude::*,
    utils::NSEC_PER_SEC,
    vulkan::VulkanApp,
};
use alga::general::SubsetOf;
use failure::{err_msg, Error};
use specs::prelude::*;
use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    ops::DerefMut,
    rc::Rc,
    time::SystemTime,
};
use winit::{dpi::LogicalSize, Event, VirtualKeyCode, WindowEvent};

pub struct GameState {
    pub resized: bool,

    pub camera: Camera,
    pub pressed_keys: HashMap<VirtualKeyCode, usize>,
    /// In pixels?
    pub mouse_delta: (f64, f64),
    /// In seconds
    pub elapsed_time: f32,
    /// In seconds
    pub frame_time: f32,
    /// In seconds
    pub fps_last_sampled_time: f32,
    /// In frames per second
    pub fps_sample: f32,
    pub camera_animation: Option<CameraAnimation>,
    pub selected: Option<Entity>,
}

pub struct Game<'a, 'b> {
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
    renderer: Rc<RefCell<Renderer>>,
}

impl<'a, 'b> Game<'a, 'b> {
    pub fn new(screen_width: u32, screen_height: u32) -> Result<Game<'a, 'b>, Error> {
        let camera = Camera::new_with_target(Point3f::new(0.0, 0.0, 3.0), Point3f::origin());
        // let camera_animation = CameraAnimation::new(
        //     &camera,
        //     Point3f::new(1.0, -1.0, 3.0),
        //     &Vector3f::new(0.0, 1.0, -3.0),
        //     1.0,
        //     1.0,
        // );

        let state = GameState {
            resized: false,
            camera,
            pressed_keys: HashMap::new(),
            mouse_delta: (0.0, 0.0),
            elapsed_time: 0.0,
            frame_time: 0.0,
            fps_sample: 0.0,
            fps_last_sampled_time: 0.0,
            // camera_animation: Some(camera_animation),
            camera_animation: None,
            selected: None,
        };
        let renderer = Rc::new(RefCell::new(VulkanApp::new(screen_width, screen_height)?));

        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();
        world.register::<BoundingBoxComponent>();
        world.add_resource(state);

        let dispatcher = {
            let mut transform_components = world.write_storage::<TransformComponent>();
            DispatcherBuilder::new()
                .with(
                    BoundingBoxComponentSystem::new(
                        transform_components.register_reader(),
                        BitSet::new(),
                        BitSet::new(),
                    ),
                    "BoundingBoxComponentSystem",
                    &[],
                )
                .with_thread_local(RenderSystem {
                    renderer: renderer.clone(),
                })
                .build()
        };

        // square
        let square = world
            .create_entity()
            .with(TransformComponent::new(
                Translation3::from(Vector3f::new(0.0, -5.0, 0.0)).to_superset(),
            ))
            .with(PrimitiveGeometryComponent::Square(Square::new(100.0)))
            .build();
        // cube 1
        let cube1 = world
            .create_entity()
            .with(TransformComponent::new(Transform3f::identity()))
            .with(PrimitiveGeometryComponent::UnitCube(UnitCube::new(1.0)))
            .build();
        // cube 2
        let cube2 = world
            .create_entity()
            .with(TransformComponent::new(
                Translation3::from(Vector3f::new(2.0, 0.0, -2.0)).to_superset(),
            ))
            .with(PrimitiveGeometryComponent::UnitCube(UnitCube::new(1.0)))
            .build();
        // cube 3
        let cube3 = world
            .create_entity()
            .with(TransformComponent::new(
                Translation3::from(Vector3f::new(-2.0, 1.0, -2.0)).to_superset(),
            ))
            .with(PrimitiveGeometryComponent::UnitCube(UnitCube::new(1.0)))
            .build();

        {
            let mut state = world.write_resource::<GameState>();
            state.selected = Some(cube1);
        }

        Ok(Game {
            world,
            dispatcher,
            renderer,
        })
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let mut running = true;
        let mut just_started = true;
        let mut focused = true;
        let mut should_grab_cursor = true;
        let mut old_cursor_grabbed = should_grab_cursor;
        let mut already_changed_cursor_state = false;
        let mut last_frame_time = SystemTime::now();
        let start_time = SystemTime::now();

        self.toggle_cursor_grab(&self.renderer.borrow_mut(), true)?;

        while running {
            let mut resized = false;
            let current_frame_time = SystemTime::now();
            let frame_time = current_frame_time.duration_since(last_frame_time)?;
            last_frame_time = current_frame_time;
            let mut mouse_delta = (0.0, 0.0);
            let mut new_cursor_grabbed = old_cursor_grabbed;
            {
                let mut state = self.world.write_resource::<GameState>();
                let state = state.deref_mut();
                let pressed_keys = &mut state.pressed_keys;
                let mut renderer = self.renderer.borrow_mut();

                renderer.events_loop().poll_events(|event| match event {
                    Event::DeviceEvent { event, .. } => {
                        if focused {
                            on_device_event(&event, pressed_keys, &mut mouse_delta);
                            if pressed_keys.contains_key(&VirtualKeyCode::Escape) {
                                running = false;
                            }

                            if let Some(&count) = pressed_keys.get(&VirtualKeyCode::F1) {
                                if count == 1 && !already_changed_cursor_state {
                                    should_grab_cursor = !should_grab_cursor;
                                    new_cursor_grabbed = should_grab_cursor;
                                    already_changed_cursor_state = true;
                                }
                            } else {
                                already_changed_cursor_state = false;
                            }
                        }
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => running = false,
                        WindowEvent::Focused(f) => {
                            focused = f;
                            if f {
                                // only regrab cursor if should_grab_cursor is toggled
                                if should_grab_cursor {
                                    new_cursor_grabbed = true;
                                }
                            } else {
                                new_cursor_grabbed = false;
                            }
                        }
                        WindowEvent::Resized(LogicalSize { width, height }) => {
                            // FIXME: handle minimization?
                            // Right now we don't allow the window to be resized
                            if just_started {
                                // When the window is first created, a resized event is sent
                                just_started = false;
                            } else {
                                println!("resized to ({}, {})", width, height);
                                resized = true;
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                });

                if new_cursor_grabbed != old_cursor_grabbed {
                    self.toggle_cursor_grab(&renderer, new_cursor_grabbed)?;
                    old_cursor_grabbed = new_cursor_grabbed;
                }
                if should_grab_cursor {
                    state.mouse_delta = mouse_delta;
                } else {
                    state.mouse_delta = (0.0, 0.0);
                }

                state.elapsed_time = start_time.elapsed()?.as_nanos() as f32 / NSEC_PER_SEC as f32;
                state.frame_time = frame_time.as_nanos() as f32 / NSEC_PER_SEC as f32;
                state.resized = resized;
            }
            self.dispatcher.dispatch(&self.world.res);
            self.world.maintain();
        }
        Ok(())
    }

    fn toggle_cursor_grab(
        &self,
        renderer: &RefMut<dyn Renderer + 'static>,
        active: bool,
    ) -> Result<(), Error> {
        let window = renderer.window();
        window.grab_cursor(active).map_err(err_msg)?;
        window.hide_cursor(active);
        Ok(())
    }
}
