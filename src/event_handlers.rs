use glutin::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode};
use std::collections::HashSet;

pub fn on_device_event(event: &DeviceEvent, pressed_keys: &mut HashSet<VirtualKeyCode>) {
    if let DeviceEvent::Key(KeyboardInput {
        virtual_keycode: Some(keycode),
        state,
        ..
    }) = event
    {
        match state {
            ElementState::Pressed => {
                pressed_keys.insert(*keycode);
            }
            ElementState::Released => {
                pressed_keys.remove(keycode);
            }
        }
    }
}
