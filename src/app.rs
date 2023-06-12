use crate::input::InputState;
use rayon::prelude::*;
use std::time::Duration;

pub trait Actor {
    fn update(&mut self, dt: &Duration, input_state: &InputState);
}
pub trait Model {
    fn render(&mut self);
}

struct App<A: Actor, M: Model> {
    actors: Vec<A>,
    models: Vec<M>,
    input_state: InputState,
}

impl<A: Actor + Send + Sync, M: Model + Send + Sync> App<A, M> {
    pub fn new() -> Self {
        Self {
            actors: vec![],
            models: vec![],
            input_state: InputState::new(),
        }
    }

    pub fn add_model(&mut self, model: M) {
        self.models.push(model);
    }

    pub fn add_actor(&mut self, actor: A) {
        self.actors.push(actor);
    }

    pub fn update(&mut self, dt: Duration) {
        self.actors
            .par_iter_mut()
            .for_each(|actor| actor.update(&dt, &self.input_state));
        // for actor in self.actors.iter_mut() {
        //     actor.update(&dt, &self.input_state);
        // }

        self.input_state.clear();
    }

    pub fn render(&mut self) {
        self.models.par_iter_mut().for_each(|model| model.render());
        // for model in self.models.iter_mut() {
        //     model.render();
        // }
    }
}
