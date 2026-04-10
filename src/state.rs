use rand::RngExt;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoopTime {
    Sec5,
    Sec10,
    Min1,
    Day1,
    Off,
}

impl LoopTime {
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            LoopTime::Sec5 => Some(Duration::from_secs(5)),
            LoopTime::Sec10 => Some(Duration::from_secs(10)),
            LoopTime::Min1 => Some(Duration::from_secs(60)),
            LoopTime::Day1 => Some(Duration::from_secs(86400)),
            LoopTime::Off => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayOrder {
    Sequential,
    Random,
}

pub struct AppState {
    pub images: Vec<PathBuf>,
    pub current: usize,
    pub loop_time: LoopTime,
    pub play_order: PlayOrder,
}

impl AppState {
    pub fn new(images: Vec<PathBuf>) -> Self {
        Self {
            images,
            current: 0,
            loop_time: LoopTime::Sec5,
            play_order: PlayOrder::Sequential,
        }
    }

    pub fn next(&mut self) {
        if self.images.is_empty() {
            return;
        }
        match self.play_order {
            PlayOrder::Sequential => {
                self.current = (self.current + 1) % self.images.len();
            }
            PlayOrder::Random => {
                let mut rng = rand::rng();
                self.current = rng.random_range(0..self.images.len());
            }
        }
    }

    pub fn prev(&mut self) {
        if self.images.is_empty() {
            return;
        }
        match self.play_order {
            PlayOrder::Sequential => {
                if self.current == 0 {
                    self.current = self.images.len() - 1;
                } else {
                    self.current -= 1;
                }
            }
            PlayOrder::Random => {
                let mut rng = rand::rng();
                self.current = rng.random_range(0..self.images.len());
            }
        }
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.images.get(self.current)
    }
}