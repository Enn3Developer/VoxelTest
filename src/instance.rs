use crate::{frustum::Aabb, model::Vertex};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3A};
use std::mem::size_of;
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

pub const SPACE_BETWEEN: f32 = 1.0;
pub const NUM_INSTANCES_PER_ROW: u32 = 256;

pub struct Instance {
    pub position: Vec3A,
}

impl Instance {
    pub fn new<V: Into<Vec3A>>(position: V) -> Self {
        let position = position.into();
        Self { position }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        let model = Mat4::from_translation(self.position.into());
        InstanceRaw::new(model)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn new(model: Mat4) -> Self {
        Self {
            model: model.to_cols_array_2d(),
        }
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
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 8]>() as BufferAddress,
                    shader_location: 7,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 12]>() as BufferAddress,
                    shader_location: 8,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}
