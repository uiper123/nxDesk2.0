use anyhow::{bail, Result};
use std::collections::HashSet;
use std::sync::Mutex;
use crate::traits::DisplayAllocator;

pub struct LocalDisplayAllocator {
    min_display: u8,
    max_display: u8,
    allocated: Mutex<HashSet<u8>>,
}

impl LocalDisplayAllocator {
    pub fn new(min: u8, max: u8) -> Self {
        Self {
            min_display: min,
            max_display: max,
            allocated: Mutex::new(HashSet::new()),
        }
    }
}

impl DisplayAllocator for LocalDisplayAllocator {
    fn allocate(&self) -> Result<u8> {
        let mut allocated = self.allocated.lock().unwrap();
        for display_id in self.min_display..=self.max_display {
            // Check if X11 socket already exists in filesystem (e.g. /tmp/.X11-unix/X<id>)
            let socket_path = format!("/tmp/.X11-unix/X{}", display_id);
            if !allocated.contains(&display_id) && !std::path::Path::new(&socket_path).exists() {
                allocated.insert(display_id);
                return Ok(display_id);
            }
        }
        bail!("No available displays in range {}-{}", self.min_display, self.max_display)
    }

    fn release(&self, display_id: u8) {
        let mut allocated = self.allocated.lock().unwrap();
        allocated.remove(&display_id);
    }
}
