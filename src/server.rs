use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContext, egui};
use egui::Align2;
use lightyear::prelude::server::Replicate;
use lightyear::prelude::*;
use lightyear::server::events::{ConnectEvent, DisconnectEvent};
use rust_i18n::t;
use server::{
    InputEvent, IoConfig, NetConfig, NetcodeConfig, ServerCommands, ServerConfig, ServerPlugins,
    ServerTransport,
};

use crate::player::{PlayerAction, PlayerId, PlayerSpawner, ProjectileSprite, ScoreMarker};
use crate::shared::{DefaultChannel, StartGameMessage};
use crate::{
    ACC_SPEED, CircleCollider, CleanupOnGameOver, MAX_VELOCITY, PROJECTILE_SPEED, ROTATION_SPEED,
    Velocity, WINDOW_HEIGHT, WINDOW_WIDTH, WrapTimeout,
};
use crate::{
    GameState, HostGame, SERVER_ADDR, ServerAddress,
    shared::{self, SERVER_REPLICATION_INTERVAL},
};

pub struct ServerPlugin;

fn net_config(address: SocketAddr) -> NetConfig {
    let io = IoConfig {
        transport: ServerTransport::UdpSocket(address),
        ..default()
    };
    NetConfig::Netcode {
        io,
        config: NetcodeConfig::default(),
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let config = ServerConfig {
            shared: shared::shared_config(),
            net: vec![net_config(SERVER_ADDR)],
            replication: ReplicationConfig {
                send_interval: SERVER_REPLICATION_INTERVAL,
                ..default()
            },
            ..default()
        };
        app.add_plugins(ServerPlugins::new(config))
            .add_observer(on_host_game)
            .add_observer(on_start_game)
            .init_resource::<ConnectedPlayers>()
            .add_systems(
                OnEnter(GameState::Playing),
                spawn_player_for_each_connection,
            )
            .add_systems(FixedUpdate, handle_player_inputs.run_if(is_server))
            .add_observer(shoot_projectile)
            .add_systems(
                Update,
                (
                    (
                        lobby_menu,
                        handle_connections,
                        handle_disconnections,
                        lobby_menu,
                    )
                        .run_if(in_state(GameState::Lobby).and(is_server)),
                    update_server_config.run_if(in_state(GameState::MainMenu)),
                ),
            );
    }
}

#[derive(Resource, Default)]
struct ConnectedPlayers {
    players: Vec<u64>,
}

fn spawn_player_for_each_connection(
    mut cmd: Commands,
    players: Res<ConnectedPlayers>,
    spawner: Single<&PlayerSpawner>,
) {
    for player in &players.players {
        cmd.spawn((
            spawner.player_client(),
            PlayerId(*player),
            Transform::from_xyz(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0, 0.0),
            Velocity { x: 0.0, y: 0.0 },
            Replicate::default(),
        ));
    }
}

fn handle_player_inputs(
    mut inputs: EventReader<InputEvent<PlayerAction>>,
    mut players: Query<(&PlayerId, Entity, &mut Transform, &mut Velocity)>,
    mut cmd: Commands,
    time: Res<Time>,
) {
    for input in inputs.read() {
        if input.input().is_none() {
            continue;
        }
        let action_state = input.input().as_ref().unwrap();
        let player = players
            .iter_mut()
            .find(|it| it.0.0 == input.from().to_bits());
        if let Some((_, e, mut transform, mut velocity)) = player {
            let direction = transform.rotation * Vec3::Y;
            let translation = direction * ACC_SPEED * time.delta().as_secs_f32();
            match action_state {
                PlayerAction::Forward => velocity.update(translation.xy()),
                PlayerAction::Shoot => cmd.trigger(NetworkPlayerShoot(e)),
                PlayerAction::Rotate(sign) => {
                    transform.rotate_z(-1.0 * *sign as f32 * ROTATION_SPEED * time.delta_secs())
                }
                PlayerAction::None => (),
            }
            velocity.max(MAX_VELOCITY);
        }
    }
}

#[derive(Event)]
struct NetworkPlayerShoot(Entity);

fn shoot_projectile(
    trigger: Trigger<NetworkPlayerShoot>,
    players: Query<(&Transform, &Velocity), With<PlayerId>>,
    mut cmd: Commands,
    material: Res<ProjectileSprite>,
) {
    let id = trigger.event().0;
    let (transform, velocity) = players.get(id).unwrap();
    let direction = transform.rotation * Vec3::Y;
    cmd.spawn((
        Transform::from_translation(transform.translation),
        Mesh2d(material.1.clone()),
        MeshMaterial2d(material.0.clone()),
        Velocity {
            x: velocity.x + direction.x * PROJECTILE_SPEED,
            y: velocity.y + direction.y * PROJECTILE_SPEED,
        },
        WrapTimeout(1),
        CircleCollider::new(10.0),
        ScoreMarker,
        CleanupOnGameOver,
        Replicate::default(),
    ));
}

fn handle_connections(
    mut connections: EventReader<ConnectEvent>,
    mut players: ResMut<ConnectedPlayers>,
) {
    for connection in connections.read() {
        players.players.push(connection.client_id.to_bits());
    }
}

fn handle_disconnections(
    mut connections: EventReader<DisconnectEvent>,
    mut players: ResMut<ConnectedPlayers>,
) {
    for connection in connections.read() {
        players
            .players
            .retain(|&id| id != connection.client_id.to_bits());
    }
}

#[derive(Event)]
struct StartGame;

fn lobby_menu(
    mut cmd: Commands,
    mut ctx: Query<&mut EguiContext, With<PrimaryWindow>>,
    players: Res<ConnectedPlayers>,
) {
    let Ok(mut ctx) = ctx.get_single_mut() else {
        return;
    };
    ctx.get_mut()
        .options_mut(|opt| opt.warn_on_id_clash = false); // Likely irrelevant warning
    let rect = ctx.get_mut().input(|i: &egui::InputState| i.screen_rect());
    egui::Window::new("Lobby")
        .pivot(Align2::CENTER_CENTER)
        .current_pos(egui::Pos2::new(rect.max.x / 2.0, rect.max.y / 2.0))
        .show(ctx.get_mut(), |ui| {
            for client in &players.players {
                ui.label(format!("Player {}", client));
            }
            if ui.button(t!("play")).clicked() {
                cmd.trigger(StartGame);
            }
        });
}

fn on_start_game(
    _trigger: Trigger<StartGame>,
    mut server: ResMut<server::ConnectionManager>,
    mut state: ResMut<NextState<GameState>>,
) {
    server
        .send_message_to_target::<DefaultChannel, StartGameMessage>(
            &StartGameMessage,
            NetworkTarget::All,
        )
        .unwrap_or_else(|e| {
            error!("Failed to send start game message: {}", e);
        });
    state.set(GameState::Playing);
}

fn on_host_game(
    _trigger: Trigger<HostGame>,
    mut cmd: Commands,
    mut state: ResMut<NextState<GameState>>,
) {
    cmd.start_server();
    state.set(GameState::Lobby);
}

fn update_server_config(mut server_config: ResMut<ServerConfig>, address: Res<ServerAddress>) {
    if address.is_changed() {
        let address = SocketAddr::new(
            address
                .ip
                .parse()
                .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            address.port,
        );
        server_config.net = vec![net_config(address)];
    }
}
