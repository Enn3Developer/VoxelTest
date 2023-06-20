use std::{
    cell::RefCell,
    mem::{self, size_of},
    rc::Rc,
};

use bytemuck::{Pod, Zeroable};
use glam::{UVec3, Vec3, Vec3A};
use uuid::Uuid;
use wgpu::{
    BindGroupLayoutEntry, BindingType, BufferAddress, BufferBindingType, BufferUsages,
    ShaderStages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::{
    app::Model,
    command_buffer::{CommandBuffer, NCommandRender, NCommandSetup, NResource},
    frustum::Aabb,
    model::Vertex,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Block {
    data: u32,
}

impl Vertex for Block {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Block>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Uint32,
                },
                VertexAttribute {
                    offset: size_of::<u32>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl Block {
    pub fn new(data: u32) -> Self {
        Self { data }
    }

    pub fn with_position<V: Into<UVec3>>(mut self, position: V) -> Self {
        let position: UVec3 = position.into();
        let pos = position.x << 6 | position.y << 3 | position.z;
        let data = self.data >> 9;
        self.data = data << 9 | pos;

        self
    }

    pub fn with_id(mut self, id: u16) -> Self {
        let position = self.data & 0b111111111;
        let data = self.data >> 25;
        self.data = data << 25 | (id as u32) << 9 | position;

        self
    }

    pub fn x(&self) -> u32 {
        (self.data & 0b111000000) >> 6
    }

    pub fn y(&self) -> u32 {
        (self.data & 0b111000) >> 3
    }

    pub fn z(&self) -> u32 {
        self.data & 0b111
    }

    pub fn position(&self) -> NVec {
        let position = self.data & 0b111111111;
        UVec3 {
            x: position >> 6,
            y: (position >> 3) & 0b111,
            z: position & 0b111,
        }
        .into()
    }

    pub fn id(&self) -> u16 {
        ((self.data >> 9) & 0xffff) as u16
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
pub struct NVec {
    x: u32,
    y: u32,
    z: u32,
    _padding: u32,
}

impl NVec {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            x,
            y,
            z,
            _padding: 0,
        }
    }
}

impl<V: Into<UVec3>> From<V> for NVec {
    fn from(value: V) -> Self {
        let vec: UVec3 = value.into();

        Self {
            x: vec.x,
            y: vec.y,
            z: vec.z,
            _padding: 0,
        }
    }
}

impl Into<Vec3> for NVec {
    fn into(self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }
}

impl Into<Vec3A> for NVec {
    fn into(self) -> Vec3A {
        Vec3A::new(self.x as f32, self.y as f32, self.z as f32)
    }
}

impl Into<Vec3A> for &NVec {
    fn into(self) -> Vec3A {
        Vec3A::new(self.x as f32, self.y as f32, self.z as f32)
    }
}

pub struct Chunk {
    id: Uuid,
    position: NVec,
    aabb: Aabb,
    blocks: Vec<Block>,
    block_data: Rc<RefCell<Vec<u8>>>,
}

impl Chunk {
    pub fn new(id: Uuid, position: NVec) -> Self {
        Self {
            id,
            position,
            aabb: Aabb::from_params(position.into(), (Into::<Vec3>::into(position) + 8.0).into()),
            blocks: vec![],
            block_data: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn exists_block<V: Into<NVec>>(&self, position: V) -> bool {
        let position: NVec = position.into();
        for block in &self.blocks {
            if block.position() == position {
                return true;
            }
        }

        false
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    pub fn add_block_data<V: Into<UVec3>>(&mut self, position: V, id: u16) {
        self.blocks
            .push(Block::default().with_position(position).with_id(id));
    }

    pub fn remove_block<V: Into<NVec>>(&mut self, position: V) {
        let position: NVec = position.into();
        let mut idx = None;
        for (i, block) in self.blocks.iter().enumerate() {
            if block.position() == position {
                idx = Some(i);
            }
        }

        if let Some(i) = idx {
            self.blocks.swap_remove(i);
        }
    }
}

impl Model for Chunk {
    fn id(&self) -> &Uuid {
        &self.id
    }

    fn aabb(&self) -> &Aabb {
        &self.aabb
    }

    fn position(&self) -> &NVec {
        &self.position
    }

    fn setup(&self) -> CommandBuffer<NCommandSetup> {
        let mut buffer = CommandBuffer::new();

        let position_buffer = Rc::new(RefCell::new(
            bytemuck::cast_slice::<_, u8>(&[self.position]).to_vec(),
        ));

        let mut data = self.block_data.borrow_mut();
        data.clear();
        for b in bytemuck::cast_slice(&self.blocks) {
            data.push(*b);
        }

        buffer.push(NCommandSetup::CreateBuffer(
            position_buffer,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        ));
        buffer.push(NCommandSetup::CreateBuffer(
            self.block_data.clone(),
            BufferUsages::VERTEX,
        ));
        buffer.push(NCommandSetup::CreateBindGroup(
            vec![BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            vec![NResource::Buffer(0)],
        ));
        buffer.push(NCommandSetup::CreatePipeline(
            vec![0],
            include_str!("../shaders/chunk_instance.wgsl"),
            vec![Block::desc()],
            true,
        ));

        buffer
    }

    fn render(&self) -> CommandBuffer<NCommandRender> {
        todo!()
    }
}
