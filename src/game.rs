use crate::{
    camera::Camera,
    ecs::{
        BoundingBoxComponent, BoundingBoxComponentSystem, PrimitiveGeometryComponent,
        RenderSystem2, TransformComponent,
    },
    event_handlers::on_device_event,
    renderer::Renderer,
    types::*,
    vulkan::VulkanBase,
};
use failure::Error;
use specs::prelude::*;
use std::{cell::RefCell, collections::HashSet, ops::DerefMut, rc::Rc};
use winit::{dpi::LogicalSize, DeviceEvent, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct GameState {
    pub resized: bool,

    pub camera: Camera,
    pub pressed_keys: HashSet<VirtualKeyCode>,
    pub mouse_delta: (f64, f64),
}

pub struct Game<'a, 'b> {
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
    renderer: Rc<RefCell<Renderer>>,
}

impl<'a, 'b> Game<'a, 'b> {
    pub fn new() -> Result<Game<'a, 'b>, Error> {
        let state = GameState {
            resized: false,
            camera: Camera::new_with_target(Point3f::new(-3.0, 3.0, -3.0), Point3f::origin()),
            pressed_keys: HashSet::new(),
            mouse_delta: (0.0, 0.0),
        };
        let renderer = Rc::new(RefCell::new(VulkanBase::new(1024, 768)?));

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
                        transform_components.track_inserted(),
                        transform_components.track_modified(),
                        BitSet::new(),
                        BitSet::new(),
                    ),
                    "BoundingBoxComponentSystem",
                    &[],
                )
                .with_thread_local(RenderSystem2 {
                    renderer: renderer.clone(),
                })
                .build()
        };

        // // square
        // world
        //     .create_entity()
        //     .with(TransformComponent::new(
        //         Translation3::from_vector(Vector3f::new(0.0, -5.0, 0.0)).to_superset(),
        //     ))
        //     .with(PrimitiveGeometryComponent::Square(Square::new(100.0)))
        //     .build();
        // // cube 1
        // world
        //     .create_entity()
        //     .with(TransformComponent::new(Transform3f::identity()))
        //     .with(PrimitiveGeometryComponent::UnitCube(UnitCube::new(1.0)))
        //     .build();
        // // cube 2
        // let cube3 = world
        //     .create_entity()
        //     .with(TransformComponent::new(
        //         Translation3::from_vector(Vector3f::new(2.0, 0.0, -2.0)).to_superset(),
        //     ))
        //     .with(PrimitiveGeometryComponent::UnitCube(UnitCube::new(1.0)))
        //     .build();
        // // cube 3
        // world
        //     .create_entity()
        //     .with(TransformComponent::new(
        //         Translation3::from_vector(Vector3f::new(-2.0, 1.0, -2.0)).to_superset(),
        //     ))
        //     .with(PrimitiveGeometryComponent::UnitCube(UnitCube::new(1.0)))
        //     .build();

        Ok(Game {
            world,
            dispatcher,
            renderer,
        })
    }

    pub fn start(&mut self) {
        let mut running = true;
        let mut just_started = true;
        let mut focused = true;
        while running {
            let mut resized = false;
            let mut mouse_delta = (0.0, 0.0);
            {
                let mut state = self.world.write_resource::<GameState>();
                let state = state.deref_mut();
                let pressed_keys = &mut state.pressed_keys;
                self.renderer
                    .borrow_mut()
                    .events_loop()
                    .poll_events(|event| match event {
                        Event::DeviceEvent { event, .. } => {
                            on_device_event(&event, pressed_keys, &mut mouse_delta);
                            if pressed_keys.contains(&VirtualKeyCode::Escape) && focused {
                                running = false;
                            }
                        }
                        Event::WindowEvent { event, .. } => match event {
                            WindowEvent::CloseRequested => running = false,
                            WindowEvent::Focused(f) => focused = f,
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
                state.mouse_delta = mouse_delta;
                state.resized = resized;
            }
            self.dispatcher.dispatch(&self.world.res);
            self.world.maintain();
        }
    }
}
