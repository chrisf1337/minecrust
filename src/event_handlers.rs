use std::collections::HashMap;
use winit::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode};

pub fn on_device_event(
    event: &DeviceEvent,
    pressed_keys: &mut HashMap<VirtualKeyCode, usize>,
    (delta_x, delta_y): &mut (f64, f64),
) {
    match event {
        DeviceEvent::Key(KeyboardInput {
            virtual_keycode: Some(keycode),
            state,
            ..
        }) => match state {
            ElementState::Pressed => {
                *pressed_keys.entry(*keycode).or_insert(0) += 1;
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
