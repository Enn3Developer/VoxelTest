use std::{slice::Iter, vec::IntoIter};

use glam::Vec3A;

use crate::app::{Actor, Model};

pub enum NCommand {
    CreateModel(Box<dyn Model>),
    CreateActor(Box<dyn Actor>),
    RemoveModel(usize),
    RemoveActor(usize),
}

pub struct CommandBuffer {
    commands: Vec<NCommand>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    pub fn push(&mut self, command: NCommand) {
        self.commands.push(command);
    }

    pub fn iter_command(self) -> IntoIter<NCommand> {
        self.commands.into_iter()
    }
}
