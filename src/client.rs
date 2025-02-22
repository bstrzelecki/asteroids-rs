use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use client::{ClientCommands, ClientTransport, IoConfig, NetConfig, NetcodeConfig};
use lightyear::prelude::*;
use lightyear::shared::events::components::{EntitySpawnEvent, MessageEvent};
use lightyear::{
    client::{config::ClientConfig, plugin::ClientPlugins},
    prelude::client::Authentication,
};
use rust_i18n::t;

use crate::asteroid::{AsteroidSpawner, LargeAsteroid};
use crate::{
    CircleCollider, CleanupOnGameOver, CleanupOnGameStart, GameState, JoinGame,
    LARGE_ASTEROID_RADIUS, SERVER_ADDR, SMALL_ASTEROID_RADIUS, ServerAddress, Velocity,
    WrapTimeout, shared,
};

pub struct ClientPlugin;

pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

fn net_config(address: SocketAddr, id: u64) -> NetConfig {
    let auth = Authentication::Manual {
        server_addr: address,
        client_id: id,
        private_key: Key::default(),
        protocol_id: 0,
    };
    let io = IoConfig {
        transport: ClientTransport::UdpSocket(CLIENT_ADDR),
        ..default()
    };
    NetConfig::Netcode {
        auth,
        io,
        config: NetcodeConfig::default(),
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        let id = rand::random::<u64>(); // Use proper rng
        let config = ClientConfig {
            shared: shared::shared_config(),
            net: net_config(SERVER_ADDR, id),
            ..default()
        };
        app.add_plugins(ClientPlugins::new(config));
        app.add_observer(on_join_game)
            .add_systems(OnEnter(GameState::Lobby), on_join_lobby)
            .add_systems(
                Update,
                (
                    update_client_config.run_if(in_state(GameState::MainMenu)),
                    wait_for_start.run_if(in_state(GameState::Lobby)),
                    on_asteroid_spawn,
                ),
            );
    }
}

fn wait_for_start(
    mut events: EventReader<MessageEvent<shared::StartGameMessage>>,
    mut state: ResMut<NextState<GameState>>,
) {
    for _ in events.read() {
        state.set(GameState::Playing);
    }
}

fn on_join_lobby(mut cmd: Commands) {
    cmd.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Center,
            padding: UiRect {
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(10.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        CleanupOnGameStart,
    ))
    .with_child((Text::new(t!("waiting.for.host")),));
}

fn on_asteroid_spawn(
    mut events: EventReader<EntitySpawnEvent>,
    asteroids: Query<(&Transform, &Velocity, Option<&LargeAsteroid>)>,
    mut cmd: Commands,
    spawner: Single<&AsteroidSpawner>,
) {
    for event in events.read() {
        let is_large = asteroids.get(event.entity()).is_ok_and(|it| it.2.is_some());
        cmd.entity(event.entity()).insert((
            spawner.asteroid_client(is_large),
            CircleCollider::new(if is_large {
                LARGE_ASTEROID_RADIUS
            } else {
                SMALL_ASTEROID_RADIUS
            }),
        ));
    }
}

fn on_join_game(
    _trigger: Trigger<JoinGame>,
    mut cmd: Commands,
    mut state: ResMut<NextState<GameState>>,
) {
    cmd.connect_client();
    state.set(GameState::Lobby);
}

fn update_client_config(mut client_config: ResMut<ClientConfig>, address: Res<ServerAddress>) {
    if address.is_changed() {
        let address = SocketAddr::new(
            address
                .ip
                .parse()
                .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            address.port,
        );
        let id = rand::random::<u64>(); // Use proper rng
        client_config.net = net_config(address, id)
    }
}
