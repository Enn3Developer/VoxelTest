use glam::Vec3A;
use std::vec::IntoIter;
use uuid::Uuid;

use crate::app::{Actor, Model};

pub trait NCommand {}

pub enum NCommandUpdate {
    CreateModel(Box<dyn Model>),
    CreateActor(Box<dyn Actor>),
    RemoveModel(Uuid),
    RemoveActor(Uuid),
    MoveCamera(Vec3A),
    RotateCamera(f32, f32),
    FovCamera(f32),
}

impl NCommand for NCommandUpdate {}

pub enum NCommandSetup {
    CreatePipeline(/*TODO: add the necessary data*/),
}

impl NCommand for NCommandSetup {}

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
