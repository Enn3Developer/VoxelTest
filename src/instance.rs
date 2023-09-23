use crate::model::Vertex;
use bytemuck::{Pod, Zeroable};
use glam::{Vec3A, Vec4};
use std::mem::size_of;
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

pub struct Instance {
    pub position: Vec3A,
}

impl Instance {
    pub fn new<V: Into<Vec3A>>(position: V) -> Self {
        let position = position.into();
        Self { position }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        let model = Vec4::new(self.position.x, self.position.y, self.position.z, 1.0);
        InstanceRaw::new(model)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceRaw {
    model: [f32; 4],
}

impl InstanceRaw {
    pub fn new(model: Vec4) -> Self {
        Self {
            model: model.to_array(),
        }
    }
}

impl Vertex for InstanceRaw {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 5,
                format: VertexFormat::Float32x4,
            }],
        }
    }
}
