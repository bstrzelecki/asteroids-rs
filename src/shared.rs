use std::time::Duration;

use bevy::prelude::*;
use lightyear::prelude::*;

use serde::{Deserialize, Serialize};

use crate::{Velocity, asteroid::LargeAsteroid, player::PlayerId};

pub struct SharedPlugin;

pub const SERVER_REPLICATION_INTERVAL: Duration = Duration::from_millis(100);
pub const FIXED_TIMESTEP_HZ: f64 = 64.0;

pub fn shared_config() -> SharedConfig {
    SharedConfig {
        server_replication_send_interval: SERVER_REPLICATION_INTERVAL,
        tick: TickConfig {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        },
        mode: Mode::HostServer,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartGameMessage;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollisionMessage {
    pub entity1: Entity,
    pub entity2: Entity,
}

#[derive(Channel)]
pub struct DefaultChannel;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.register_message::<StartGameMessage>(ChannelDirection::ServerToClient);
        app.add_channel::<DefaultChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        });
        app.register_component::<Transform>(ChannelDirection::ServerToClient);
        app.register_component::<Velocity>(ChannelDirection::ServerToClient);
        app.register_component::<LargeAsteroid>(ChannelDirection::ServerToClient);
        app.register_component::<PlayerId>(ChannelDirection::ServerToClient);
    }
}
