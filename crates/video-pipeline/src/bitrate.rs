use crate::traits::BitrateController;

pub struct AdaptiveBitrateController {
    current_bitrate_kbps: u32,
    min_bitrate_kbps: u32,
    max_bitrate_kbps: u32,
    target_latency_ms: u64,
}

impl AdaptiveBitrateController {
    pub fn new(min: u32, max: u32, initial: u32, target_latency: u64) -> Self {
        Self {
            current_bitrate_kbps: initial,
            min_bitrate_kbps: min,
            max_bitrate_kbps: max,
            target_latency_ms: target_latency,
        }
    }
}

impl BitrateController for AdaptiveBitrateController {
    fn calculate_bitrate(&mut self, current_latency_ms: u64, packet_loss_rate: f64) -> u32 {
        if packet_loss_rate > 0.05 || current_latency_ms > self.target_latency_ms * 2 {
            // Multiplicative decrease (back off by 20%)
            self.current_bitrate_kbps = (self.current_bitrate_kbps as f64 * 0.8) as u32;
        } else if current_latency_ms <= self.target_latency_ms && packet_loss_rate < 0.01 {
            // Additive increase (+200 kbps)
            self.current_bitrate_kbps += 200;
        }

        // Clamp to min/max
        if self.current_bitrate_kbps < self.min_bitrate_kbps {
            self.current_bitrate_kbps = self.min_bitrate_kbps;
        }
        if self.current_bitrate_kbps > self.max_bitrate_kbps {
            self.current_bitrate_kbps = self.max_bitrate_kbps;
        }

        self.current_bitrate_kbps
    }

    fn current_bitrate(&self) -> u32 {
        self.current_bitrate_kbps
    }
}
