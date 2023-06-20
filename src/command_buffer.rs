use glam::Vec3A;
use std::{cell::RefCell, rc::Rc, vec::IntoIter};
use uuid::Uuid;
use wgpu::{BindGroupLayoutEntry, BufferUsages, IndexFormat, VertexBufferLayout};

use crate::app::{Actor, Model};

pub type Index = usize;
pub type ID = Uuid;
pub type NModel = Box<dyn Model + Send + Sync>;
pub type NActor = Box<dyn Actor + Send>;

pub trait NCommand {}

pub enum NResource {
    Buffer(Index),
}

pub enum NCommandUpdate {
    CreateModel(NModel),
    CreateActor(NActor),
    RemoveModel(ID),
    RemoveActor(ID),
    MoveCamera(Vec3A),
    RotateCamera(f32, f32),
    FovCamera(f32),
    UpdateBuffer(ID, Index),
}

impl NCommand for NCommandUpdate {}

pub enum NCommandSetup {
    CreateBuffer(Rc<RefCell<Vec<u8>>>, BufferUsages),
    CreateBindGroup(Vec<BindGroupLayoutEntry>, Vec<NResource>),
    CreatePipeline(Vec<Index>, &'static str, Vec<VertexBufferLayout<'static>>, bool),
    SharePipeline(&'static ID, Index),
}

impl NCommand for NCommandSetup {}

pub enum NCommandRender {
    SetPipeline(Index),
    SetVertexBuffer(u32, Index),
    SetIndexBuffer(Index, IndexFormat),
    SetBindGroup(u32, Index),
    DrawIndexed(u32, u32),
    DrawModelIndexed(Index, u32, &'static [Index]),
}

impl NCommand for NCommandRender {}

pub struct CommandBuffer<N: NCommand> {
    commands: Vec<N>,
}

impl<N: NCommand> CommandBuffer<N> {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    pub fn push(&mut self, command: N) {
        self.commands.push(command);
    }

    pub fn iter_command(self) -> IntoIter<N> {
        self.commands.into_iter()
    }
}
