use std::sync::Arc;

pub struct ImageLoader;

impl ImageLoader {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn queue_image_load<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        rayon::spawn(task);
    }
}
