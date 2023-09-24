use crate::model::Vertex;
use bytemuck::{Pod, Zeroable};
use glam::{Vec3A, Vec4};
use std::mem::size_of;
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

#[derive(Debug)]
pub struct Instance {
    pub position: Vec3A,
    pub id: u16,
}

impl Instance {
    pub fn new<V: Into<Vec3A>>(position: V, id: u16) -> Self {
        let position = position.into();
        Self { position, id }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        let model = Vec4::new(self.position.x, self.position.y, self.position.z, 1.0);
        InstanceRaw::new(model, self.id as u32)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceRaw {
    model: [f32; 4],
    id: u32,
}

impl InstanceRaw {
    pub fn new(model: Vec4, id: u32) -> Self {
        Self {
            model: model.to_array(),
            id,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl Vertex for InstanceRaw {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 6,
                    format: VertexFormat::Uint32,
                },
            ],
        }
    }
}
