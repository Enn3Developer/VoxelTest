use winit::event::WindowEvent;

// TODO: Implement all the needed functions

pub struct InputState {}

impl InputState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn clear(&mut self) {}

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }
}
