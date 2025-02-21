use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContext, EguiContexts, egui};
use egui::Align2;
use lightyear::prelude::*;
use lightyear::server::events::{ConnectEvent, DisconnectEvent};
use server::{
    IoConfig, NetConfig, NetcodeConfig, ServerCommands, ServerConfig, ServerPlugins,
    ServerTransport,
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
            .init_resource::<ConnectedPlayers>()
            .add_systems(
                Update,
                (
                    (
                        lobby_menu,
                        handle_connections,
                        handle_disconnections,
                        lobby_menu,
                    )
                        .run_if(in_state(GameState::Lobby)),
                    update_server_config.run_if(in_state(GameState::MainMenu)),
                ),
            );
    }
}

#[derive(Resource, Default)]
struct ConnectedPlayers {
    players: Vec<u64>,
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

fn lobby_menu(
    mut cmd: Commands,
    mut ctx: Query<&mut EguiContext, With<PrimaryWindow>>,
    players: Res<ConnectedPlayers>,
) {
    let Ok(mut ctx) = ctx.get_single_mut() else {
        return;
    };
    let rect = ctx.get_mut().input(|i: &egui::InputState| i.screen_rect());
    egui::Window::new("Lobby")
        .pivot(Align2::CENTER_CENTER)
        .current_pos(egui::Pos2::new(rect.max.x / 2.0, rect.max.y / 2.0))
        .show(ctx.get_mut(), |ui| {
            for client in &players.players {
                ui.label(format!("Player {}", client));
            }
        });
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
