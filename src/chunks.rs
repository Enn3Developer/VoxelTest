use glam::{UVec3, Vec3A};
use uuid::Uuid;

use crate::{
    app::Model,
    command_buffer::{CommandBuffer, NCommandRender, NCommandSetup},
    frustum::Aabb,
};

pub struct Block {
    data: u32,
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

    pub fn position(&self) -> UVec3 {
        let position = self.data & 0b111111111;
        UVec3 {
            x: position >> 6,
            y: (position >> 3) & 0b111,
            z: position & 0b111,
        }
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

pub struct Chunk {
    id: Uuid,
    position: Vec3A,
    aabb: Aabb,
    blocks: Vec<Block>,
}

impl Chunk {
    pub fn new(id: Uuid, position: Vec3A) -> Self {
        Self {
            id,
            position,
            aabb: Aabb::from_params(position.into(), (position + 8.0).into()),
            blocks: vec![],
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
    }

    pub fn add_block_data<V: Into<UVec3>>(&mut self, position: V, id: u16) {
        self.blocks
            .push(Block::default().with_position(position).with_id(id));
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
        todo!()
    }

    fn render(&self) -> CommandBuffer<NCommandRender> {
        todo!()
    }
}
