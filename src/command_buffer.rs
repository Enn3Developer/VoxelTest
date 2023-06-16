use bytemuck::{Pod, Zeroable};
use glam::Vec3A;
use std::{cell::RefCell, rc::Rc, vec::IntoIter};
use uuid::Uuid;
use wgpu::{BindGroupLayoutEntry, BufferUsages};

use crate::app::{Actor, Model};

pub trait NUniform: Pod + Zeroable {}

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

pub enum NCommandSetup<N: NUniform> {
    CreateBuffer(Rc<RefCell<N>>, BufferUsages),
    CreateBindGroup(&'static [BindGroupLayoutEntry], Vec<NResource>),
    CreatePipeline(/*TODO: add the necessary data*/),
}

impl<T> NUniform for T where T: Pod + Zeroable {}

impl<N: NUniform> NCommand for NCommandSetup<N> {}

pub enum NCommandRender {
    // TODO: add all the possible commands
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
