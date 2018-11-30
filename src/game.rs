use crate::{game_state::GameState, renderer::Renderer, vulkan::VulkanBase};
use failure::Error;
use winit::{dpi::LogicalSize, DeviceEvent, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct Game {
    state: GameState,
    renderer: Box<Renderer>,
}

impl Game {
    pub fn new() -> Result<Game, Error> {
        let renderer = Box::new(VulkanBase::new(1024, 768)?);
        let state = GameState {};
        Ok(Game { state, renderer })
    }

    pub fn start(&mut self) {
        let mut running = true;
        let mut just_started = true;
        while running {
            let mut resized = false;
            self.renderer
                .events_loop()
                .poll_events(|event| match event {
                    Event::DeviceEvent { event, .. } => match event {
                        DeviceEvent::Key(KeyboardInput {
                            virtual_keycode: Some(keycode),
                            ..
                        }) => {
                            if keycode == VirtualKeyCode::Escape {
                                running = false;
                            }
                        }
                        _ => (),
                    },
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => running = false,
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
            unsafe {
                let _ = self.renderer.draw_frame(&self.state, resized);
            }
        }
    }
}
