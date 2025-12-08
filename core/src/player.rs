use crate::Backend;

pub struct Player {
    backend: Backend,
}

impl Player {
    pub fn new() -> Self {
       Player {
            backend: Backend::new(),
        } 
    }

    pub fn play() {

    }

    pub fn pause() {

    }

    pub fn seek(pos: f32) {
        
    }
}
