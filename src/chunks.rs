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

#[derive(Debug)]
pub struct Block {
    id: u16,
    instance: Instance,
}

impl Block {
    pub fn new(position: Vec3A) -> Self {
        Self {
            id: 0,
            instance: Instance::new(position),
        }
    }

    pub fn with_id(mut self, id: u16) -> Self {
        self.id = id;

        self
    }

    pub fn x(&self) -> f32 {
        self.instance.position.x
    }

    pub fn y(&self) -> f32 {
        self.instance.position.y
    }

    pub fn z(&self) -> f32 {
        self.instance.position.z
    }

    pub fn position(&self) -> &Vec3A {
        &self.instance.position
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn to_raw(&self) -> InstanceRaw {
        self.instance.to_raw()
    }
}

pub struct Chunk {
    id: Uuid,
    position: Vec3A,
    aabb: Aabb,
    blocks: Vec<Block>,
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
            block_data: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn exists_block<V: Into<Vec3A>>(&self, position: V) -> bool {
        let position: Vec3A = position.into();
        for block in &self.blocks {
            if block.position() == &position {
                return true;
            }
        }

        false
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    pub fn add_block_data<V: Into<Vec3A>>(&mut self, position: V, id: u16) {
        self.add_block(Block::new(position.into() + self.position * 16.0).with_id(id));
    }

    pub fn remove_block<V: Into<Vec3A>>(&mut self, position: V) {
        let position: Vec3A = position.into();
        let mut idx = None;
        for (i, block) in self.blocks.iter().enumerate() {
            if block.position() == &position {
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

        // let _position_buffer = Rc::new(RefCell::new(
        //     bytemuck::cast_slice::<_, u8>(&[self.position.to_array()]).to_vec(),
        // ));
        //
        // let mut data = self.block_data.borrow_mut();
        // data.clear();
        // let instances = self
        //     .instances
        //     .iter()
        //     .map(|instance| instance.to_raw())
        //     .collect::<Vec<InstanceRaw>>();
        // for b in bytemuck::cast_slice(&instances) {
        //     data.push(*b);
        // }
        //
        // buffer.push(NCommandSetup::CreateBuffer(
        //     self.block_data.clone(),
        //     BufferUsages::VERTEX,
        // ));
        // buffer.push(NCommandSetup::CreatePipeline(
        //     vec![],
        //     include_str!("../shaders/chunk_instance.wgsl"),
        //     vec![InstanceRaw::desc()],
        //     true,
        // ));

        buffer
    }

    fn render(&self) -> CommandBuffer<NCommandRender> {
        let mut buffer = CommandBuffer::new();

        // buffer.push(NCommandRender::SetPipeline(0));
        // buffer.push(NCommandRender::SetVertexBuffer(1, 0));
        // buffer.push(NCommandRender::DrawModelIndexed(
        //     0,
        //     self.blocks.len() as u32,
        //     &[],
        // ));

        buffer
    }
}

unsafe impl Send for Chunk {}
unsafe impl Sync for Chunk {}
