use std::time::Duration;

use bevy::prelude::*;
use lightyear::prelude::*;

pub struct SharedPlugin;

pub const SERVER_REPLICATION_INTERVAL: Duration = Duration::from_millis(100);
pub const FIXED_TIMESTEP_HZ: f64 = 64.0;

pub fn shared_config() -> SharedConfig {
    SharedConfig {
        server_replication_send_interval: SERVER_REPLICATION_INTERVAL,
        tick: TickConfig {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        },
        mode: Mode::Separate,
    }
}

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        todo!();
    }
}
