use std::collections::HashSet;
use std::hash::BuildHasher;
use winit::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode};

pub fn on_device_event<S: BuildHasher>(
    event: &DeviceEvent,
    pressed_keys: &mut HashSet<VirtualKeyCode, S>,
    (delta_x, delta_y): &mut (f64, f64),
) {
    match event {
        DeviceEvent::Key(KeyboardInput {
            virtual_keycode: Some(keycode),
            state,
            ..
        }) => match state {
            ElementState::Pressed => {
                pressed_keys.insert(*keycode);
            }
            ElementState::Released => {
                pressed_keys.remove(keycode);
            }
        },
        DeviceEvent::MouseMotion { delta: (dx, dy) } => {
            *delta_x += dx;
            *delta_y += dy;
        }
        _ => (),
    }
}
