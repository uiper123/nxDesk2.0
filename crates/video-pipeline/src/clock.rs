use std::time::{Duration, Instant};
use crate::traits::FrameClock;

pub struct SimpleFrameClock {
    fps: u32,
    last_tick: Instant,
}

impl SimpleFrameClock {
    pub fn new(fps: u32) -> Self {
        Self {
            fps,
            last_tick: Instant::now(),
        }
    }
}

impl FrameClock for SimpleFrameClock {
    fn tick(&mut self) -> Duration {
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps as f64);
        let elapsed = self.last_tick.elapsed();
        
        if elapsed < frame_duration {
            let sleep_dur = frame_duration - elapsed;
            std::thread::sleep(sleep_dur);
        }
        
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick);
        self.last_tick = now;
        delta
    }

    fn fps(&self) -> u32 {
        self.fps
    }
}
