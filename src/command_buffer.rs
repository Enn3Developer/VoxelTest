use glam::Vec3A;
use std::{cell::RefCell, rc::Rc, vec::IntoIter};
use uuid::Uuid;
use wgpu::{BindGroupLayoutEntry, BufferUsages, IndexFormat, VertexBufferLayout};

use crate::app::{Actor, Model};

pub trait NCommand {}

pub enum NResource {
    Buffer(usize),
}

pub enum NCommandUpdate {
    CreateModel(Box<dyn Model + Send + Sync>),
    CreateActor(Box<dyn Actor + Send>),
    RemoveModel(Uuid),
    RemoveActor(Uuid),
    MoveCamera(Vec3A),
    RotateCamera(f32, f32),
    FovCamera(f32),
}

impl NCommand for NCommandUpdate {}

pub enum NCommandSetup {
    CreateBuffer(Rc<RefCell<[u8]>>, BufferUsages),
    CreateBindGroup(&'static [BindGroupLayoutEntry], Vec<NResource>),
    CreatePipeline(
        &'static [usize],
        String,
        &'static [VertexBufferLayout<'static>],
    ),
    SharePipeline(&'static Uuid, usize),
}

impl NCommand for NCommandSetup {}

pub enum NCommandRender {
    SetVertexBuffer(u32, usize),
    SetIndexBuffer(usize, IndexFormat),
    SetBindGroup(u32, usize),
    DrawIndexed(u32, u32),
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
