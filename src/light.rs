use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightUniform {
    position: [f32; 3],
    radius: f32,
    color: [f32; 3],
    _padding2: u32,
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            color,
            radius,
            _padding2: 0,
        }
    }

    pub fn position(&self) -> &[f32; 3] {
        &self.position
    }

    pub fn set_position(&mut self, position: [f32; 3]) {
        self.position = position;
    }
}
