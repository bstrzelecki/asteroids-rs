use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use client::{ClientCommands, ClientTransport, IoConfig, NetConfig, NetcodeConfig};
use lightyear::prelude::*;
use lightyear::{
    client::{config::ClientConfig, plugin::ClientPlugins},
    prelude::client::Authentication,
};

use crate::{GameState, JoinGame, SERVER_ADDR, ServerAddress, shared};

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
        app.add_observer(on_join_game).add_systems(
            Update,
            update_client_config.run_if(in_state(GameState::MainMenu)),
        );
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
