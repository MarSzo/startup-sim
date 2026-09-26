//! Periodic server statistics (`--stats-secs`).

use std::time::Duration;

use super::Server;

/// Counters since the last report.
#[derive(Default)]
pub(super) struct Stats {
    ticks: u64,
    tick_total: Duration,
    tick_max: Duration,
    pub(super) missed: u64,
    pub(super) snapshot_bytes: u64,
    pub(super) max_visible: usize,
}

impl Stats {
    pub(super) fn record_tick(&mut self, took: Duration) {
        self.ticks += 1;
        self.tick_total += took;
        self.tick_max = self.tick_max.max(took);
    }
}

impl Server {
    /// Log the report and reset the counters.
    pub(super) fn print_stats(&mut self) {
        let secs = self.cfg.stats_every.as_secs_f64().max(f64::EPSILON);
        let s = std::mem::take(&mut self.stats);
        let avg_us = if s.ticks > 0 { s.tick_total.as_micros() as f64 / s.ticks as f64 } else { 0.0 };
        let n = self.players.len();
        let (mut min_bps, mut max_bps, mut sum) = (u64::MAX, 0u64, 0u64);
        for p in self.players.values_mut() {
            let bps = (p.bytes_out as f64 / secs) as u64;
            min_bps = min_bps.min(bps);
            max_bps = max_bps.max(bps);
            sum += bps;
            p.bytes_out = 0;
        }
        let avg_bps = sum.checked_div(n as u64).unwrap_or(0);
        if n == 0 {
            min_bps = 0;
        }
        let kb = |b: u64| b as f64 / 1024.0;
        let msg = format!(
            "tick {} | ticks {} (missed {}) | tick avg {:.0} us max {} us | players {} max_visible {} | out/client avg {:.1} KB/s (min {:.1}, max {:.1}) | in {:.0} pkt/s {:.1} KB/s | out {:.0} pkt/s | sim-dropped {}",
            self.tick,
            s.ticks,
            s.missed,
            avg_us,
            s.tick_max.as_micros(),
            n,
            s.max_visible,
            kb(avg_bps),
            kb(min_bps),
            kb(max_bps),
            self.net.packets_in as f64 / secs,
            kb(self.net.bytes_in) / secs,
            self.net.packets_out as f64 / secs,
            self.net.dropped,
        );
        self.net.reset_counters();
        self.log(msg);
    }
}
