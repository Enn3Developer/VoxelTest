use winit::event::KeyEvent;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard,
};

// TODO: Implement all the needed functions

#[derive(PartialEq, Eq)]
pub struct Key {
    keycode: keyboard::Key,
    previous: bool,
}

impl Key {
    pub fn new(keycode: keyboard::Key) -> Self {
        Self {
            keycode,
            previous: false,
        }
    }
}

pub struct InputState {
    keys: Vec<Key>,
    keys_released: Vec<keyboard::Key>,
    mouse_delta: (f32, f32),
    last_mouse_position: (f32, f32),
    mouse_sample: u32,
    mouse_scroll: f32,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys: vec![],
            keys_released: vec![],
            mouse_delta: (0.0, 0.0),
            last_mouse_position: (0.0, 0.0),
            mouse_sample: 0,
            mouse_scroll: 0.0,
        }
    }

    pub fn update(&mut self) {
        for key in self.keys.iter_mut() {
            if !key.previous {
                key.previous = true;
            }
        }

        self.keys_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_sample = 0;
        self.mouse_scroll = 0.0;
    }

    pub fn contains(&self, key: &Key) -> bool {
        for k in &self.keys {
            if k.keycode == key.keycode {
                return true;
            }
        }

        false
    }

    pub fn index(&self, key: &Key) -> usize {
        for (idx, k) in self.keys.iter().enumerate() {
            if k.keycode == key.keycode {
                return idx;
            }
        }

        0
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key, state, ..
                },
                ..
            } => {
                let key = Key::new(logical_key.clone());
                if let ElementState::Pressed = state {
                    if !self.contains(&key) {
                        self.keys.push(key);
                    }
                } else if self.contains(&key) {
                    self.keys.remove(self.index(&key));
                    self.keys_released.push(logical_key.clone());
                }

                true
            }

            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                self.mouse_delta = (
                    pos.0 - self.last_mouse_position.0,
                    pos.1 - self.last_mouse_position.1,
                );
                self.last_mouse_position = pos;
                self.mouse_sample += 1;

                true
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.mouse_scroll = match delta {
                    MouseScrollDelta::LineDelta(_, scroll) => *scroll * 100.0,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                        *scroll as f32
                    }
                };
                true
            }

            _ => false,
        }
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        if self.mouse_sample > 0 {
            (
                self.mouse_delta.0 / self.mouse_sample as f32,
                self.mouse_delta.1 / self.mouse_sample as f32,
            )
        } else {
            (0.0, 0.0)
        }
    }

    pub fn mouse_scroll(&self) -> f32 {
        self.mouse_scroll
    }

    pub fn is_key_pressed(&self, key: &keyboard::Key) -> bool {
        for k in &self.keys {
            if &k.keycode == key {
                return true;
            }
        }

        false
    }

    pub fn is_key_just_pressed(&self, key: &keyboard::Key) -> bool {
        for k in &self.keys {
            if &k.keycode == key && !k.previous {
                return true;
            }
        }

        false
    }

    pub fn is_key_just_released(&self, key: &keyboard::Key) -> bool {
        self.keys_released.contains(key)
    }
}
