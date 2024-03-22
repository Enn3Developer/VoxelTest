use std::{cell::RefCell, mem::size_of, rc::Rc};

use bytemuck::{Pod, Zeroable};
use glam::{UVec3, Vec3, Vec3A};
use uuid::Uuid;
use wgpu::{
    BufferAddress, BufferUsages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::{
    app::Model,
    command_buffer::{CommandBuffer, NCommandRender, NCommandSetup},
    frustum::Aabb,
    instance::{Instance, InstanceRaw},
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
        let pos = position.x << 8 | position.y << 4 | position.z;
        let data = self.data >> 12;
        self.data = data << 12 | pos;

        self
    }

    pub fn with_id(mut self, id: u16) -> Self {
        let position = self.data & 0b111111111111;
        let data = self.data >> 28;
        self.data = data << 28 | (id as u32) << 12 | position;

        self
    }

    pub fn x(&self) -> u32 {
        (self.data & 0b111100000000) >> 8
    }

    pub fn y(&self) -> u32 {
        (self.data & 0b11110000) >> 4
    }

    pub fn z(&self) -> u32 {
        self.data & 0b1111
    }

    pub fn position(&self) -> UVec3 {
        let position = self.data & 0b111111111111;
        UVec3 {
            x: position >> 8,
            y: (position >> 4) & 0b1111,
            z: position & 0b1111,
        }
    }

    pub fn id(&self) -> u16 {
        ((self.data >> 12) & 0xffff) as u16
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(0)
    }
}

pub struct Chunk {
    id: Uuid,
    position: Vec3A,
    aabb: Aabb,
    blocks: Vec<Block>,
    instances: Vec<Instance>,
    block_data: Rc<RefCell<Vec<u8>>>,
}

impl Chunk {
    pub fn new(id: Uuid, position: Vec3A) -> Self {
        let aabb_pos = position * Vec3A::new(16.0, 16.0, 16.0);
        Self {
            id,
            position,
            aabb: Aabb::from_params(aabb_pos.into(), Into::<Vec3>::into(aabb_pos) + 16.0),
            blocks: vec![],
            instances: vec![],
            block_data: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn exists_block<V: Into<UVec3>>(&self, position: V) -> bool {
        let position: UVec3 = position.into();
        for block in &self.blocks {
            if block.position() == position {
                return true;
            }
        }

        false
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
        let block_pos = block.position();
        self.instances.push(Instance::new(
            Vec3A::new(block_pos.x as f32, block_pos.y as f32, block_pos.z as f32)
                + (self.position * Vec3A::new(16.0, 16.0, 16.0)),
        ))
    }

    pub fn add_block_data<V: Into<UVec3>>(&mut self, position: V, id: u16) {
        self.add_block(Block::default().with_position(position).with_id(id));
    }

    pub fn remove_block<V: Into<UVec3>>(&mut self, position: V) {
        let position: UVec3 = position.into();
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

    fn position(&self) -> &Vec3A {
        &self.position
    }

    fn setup(&self) -> CommandBuffer<NCommandSetup> {
        let mut buffer = CommandBuffer::new();

        let _position_buffer = Rc::new(RefCell::new(
            bytemuck::cast_slice::<_, u8>(&[self.position.to_array()]).to_vec(),
        ));

        let mut data = self.block_data.borrow_mut();
        data.clear();
        let instances = self
            .instances
            .iter()
            .map(|instance| instance.to_raw())
            .collect::<Vec<InstanceRaw>>();
        for b in bytemuck::cast_slice(&instances) {
            data.push(*b);
        }

        buffer.push(NCommandSetup::CreateBuffer(
            self.block_data.clone(),
            BufferUsages::VERTEX,
        ));
        buffer.push(NCommandSetup::CreatePipeline(
            vec![],
            include_str!("../shaders/chunk_instance.wgsl"),
            vec![InstanceRaw::desc()],
            true,
        ));

        buffer
    }

    fn render(&self) -> CommandBuffer<NCommandRender> {
        let mut buffer = CommandBuffer::new();

        buffer.push(NCommandRender::SetPipeline(0));
        buffer.push(NCommandRender::SetVertexBuffer(1, 0));
        buffer.push(NCommandRender::DrawModelIndexed(
            0,
            self.blocks.len() as u32,
            &[],
        ));

        buffer
    }
}

unsafe impl Send for Chunk {}
unsafe impl Sync for Chunk {}
