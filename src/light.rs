use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightUniform {
    position: [f32; 3],
    _padding: u32,
    color: [f32; 3],
    _padding2: u32,
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            position,
            color,
            _padding: 0,
            _padding2: 0,
        }
    }

    pub fn position(&self) -> &[f32; 3] {
        &self.position
    }

    pub fn color(&self) -> &[f32; 3] {
        &self.color
    }

    pub fn set_position(&mut self, position: [f32; 3]) {
        self.position = position;
    }

    pub fn set_color(&mut self, color: [f32; 3]) {
        self.color = color;
    }
}
