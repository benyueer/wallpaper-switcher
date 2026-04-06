use std::path::PathBuf;

pub struct AppState {
    pub images: Vec<PathBuf>,
    pub current: usize,
    pub auto: bool,
}

impl AppState {
    pub fn new(images: Vec<PathBuf>) -> Self {
        Self {
            images,
            current: 0,
            auto: false,
        }
    }

    pub fn next(&mut self) {
        if self.images.is_empty() {
            return;
        }
        self.current = (self.current + 1) % self.images.len();
    }

    pub fn prev(&mut self) {
        if self.images.is_empty() {
            return;
        }
        if self.current == 0 {
            self.current = self.images.len() - 1;
        } else {
            self.current -= 1;
        }
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.images.get(self.current)
    }
}