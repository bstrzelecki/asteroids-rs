use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use asteroid::AsteroidPlugin;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_rand::plugin::EntropyPlugin;
use bevy_spatial::kdtree::KDTree2;
use bevy_spatial::{AutomaticUpdate, SpatialAccess, SpatialStructure, TransformMode};
use leafwing_input_manager::plugin::InputManagerPlugin;
use particles::ParticlePlugin;
use player::PlayerPlugin;
use strum::EnumIter;
use ui::UiPlugin;

mod asteroid;
mod client;
mod particles;
mod player;
mod server;
mod shared;
mod ui;

type RngType = bevy_prng::ChaCha8Rng;
const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000);

#[derive(Resource)]
struct ServerAddress {
    ip: String,
    port: u16,
}

impl Default for ServerAddress {
    fn default() -> Self {
        Self {
            ip: Ipv4Addr::LOCALHOST.to_string(),
            port: 5000,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            InputManagerPlugin::<player::PlayerAction>::default(),
            EntropyPlugin::<RngType>::default(),
            AutomaticUpdate::<SpatialMarker>::new()
                .with_frequency(Duration::from_millis(16))
                .with_spatial_ds(SpatialStructure::KDTree2)
                .with_transform(TransformMode::GlobalTransform),
        ))
        .add_plugins((PlayerPlugin, ParticlePlugin, AsteroidPlugin, UiPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    apply_velocity,
                    wrap_around,
                    check_collisions,
                    check_for_gameover,
                )
                    .run_if(in_state(GameState::Playing)),
                (handle_restart).run_if(in_state(GameState::GameOver)),
            ),
        )
        .add_systems(
            OnEnter(GameState::GameOver),
            (cleanup::<CleanupOnGameOver>,),
        )
        .add_systems(OnEnter(GameState::MainMenu), (cleanup::<CleanupOnRestart>,))
        .add_event::<CollisionEvent>()
        .init_state::<GameState>()
        .init_resource::<ServerAddress>()
        .init_resource::<Language>();

    #[cfg(feature = "client")]
    app.add_plugins((client::ClientPlugin,));

    #[cfg(feature = "server")]
    app.add_plugins((server::ServerPlugin,));

    app.run();
}

#[derive(Event)]
struct HostGame;

#[derive(Event)]
struct JoinGame;

const ACC_SPEED: f32 = 5.0;
const ROTATION_SPEED: f32 = 8.0;
const MAX_VELOCITY: f32 = 3.0;

const SHOOT_TIMEOUT: f32 = 0.5;
const PROJECTILE_SPEED: f32 = 10.0;

const WINDOW_WIDTH: f32 = 1920.0;
const WINDOW_HEIGHT: f32 = 1080.0;

const SMALL_ASTEROID_RADIUS: f32 = 20.0;
const LARGE_ASTEROID_RADIUS: f32 = 40.0;

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    MainMenu,
    Lobby,
    Playing,
    GameOver,
}

rust_i18n::i18n!("locales", fallback = "en");

#[derive(PartialEq, Default, Resource, Copy, Clone, EnumIter)]
enum Language {
    #[default]
    English,
    Polish,
    French,
}

impl Language {
    fn locale(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Polish => "pl",
            Language::French => "fr",
        }
    }
}

fn handle_restart(key: Res<ButtonInput<KeyCode>>, mut state: ResMut<NextState<GameState>>) {
    if key.just_pressed(KeyCode::KeyR) {
        state.set(GameState::MainMenu);
    }
}

#[derive(Component)]
struct CleanupOnGameOver;

#[derive(Component)]
struct CleanupOnRestart;

fn cleanup<T: Component>(mut cmd: Commands, e: Query<(Entity, &T)>) {
    e.iter().for_each(|(e, _)| {
        cmd.entity(e).despawn_recursive();
    });
}

#[derive(Component, Default)]
struct SpatialMarker;

#[derive(Component)]
#[require(SpatialMarker)]
struct CircleCollider {
    radius: f32,
}

impl CircleCollider {
    fn new(radius: f32) -> Self {
        Self { radius }
    }
}

type NNTree = KDTree2<SpatialMarker>;

#[derive(Event)]
struct CollisionEvent(Entity, Entity);

fn check_collisions(
    e: Query<(Entity, &Transform, &CircleCollider)>,
    tree: Res<NNTree>,
    mut ev_collision: EventWriter<CollisionEvent>,
) {
    e.iter().for_each(|(e, transform, col)| {
        tree.within_distance(transform.translation.xy(), col.radius)
            .iter()
            .for_each(|(_pos, entity)| {
                if let Some(other) = entity {
                    if *other == e {
                        return;
                    }
                    ev_collision.send(CollisionEvent(e, *other));
                }
            });
    });
}

fn setup(mut cmd: Commands) {
    cmd.spawn((
        Camera2d,
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0, 0.0),
    ));
}

#[derive(Component)]
struct WrapTimeout(u8);

#[derive(Component)]
struct Lives(i8);

impl Default for Lives {
    fn default() -> Self {
        Self(3)
    }
}

fn check_for_gameover(
    mut state: ResMut<NextState<GameState>>,
    lives: Option<Single<&Lives, Changed<Lives>>>,
) {
    if let Some(lives) = lives {
        if lives.0 <= 0 {
            state.set(GameState::GameOver);
        }
    }
}

#[derive(Component, Default)]
struct Score(u32);

#[derive(Event)]
struct OnScoreUpdate(u32);

fn wrap_around(
    mut e: Query<(Entity, &mut Transform, Option<&mut WrapTimeout>), With<Velocity>>,
    mut cmd: Commands,
) {
    e.iter_mut().for_each(|(e, mut it, timeout)| {
        let mut wrapped = false;
        if it.translation.x < 0.0 {
            it.translation.x = 1920.0;
            wrapped = true;
        }
        if it.translation.y < 0.0 {
            it.translation.y = 1080.0;
            wrapped = true;
        }
        if it.translation.y > 1080.0 {
            it.translation.y = 0.0;
            wrapped = true;
        }
        if it.translation.x > 1920.0 {
            it.translation.x = 0.0;
            wrapped = true;
        }
        if let Some(mut timeout) = timeout {
            if !wrapped {
                return;
            }
            if wrapped && timeout.0 == 0 {
                cmd.entity(e).despawn();
                return;
            }
            timeout.0 -= 1;
        }
    });
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Velocity {
    fn max(&mut self, val: f32) {
        if (self.x.powi(2) + self.y.powi(2)).sqrt() > val {
            let angle = self.y.atan2(self.x);
            self.x = angle.cos() * val;
            self.y = angle.sin() * val;
        }
    }
    fn update(&mut self, translation: Vec2) {
        self.x += translation.x;
        self.y += translation.y;
    }
}

fn apply_velocity(mut e: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    e.iter_mut().for_each(|mut it| {
        it.0.translation.x += it.1.x * time.delta_secs() * 100.0;
        it.0.translation.y += it.1.y * time.delta_secs() * 100.0;
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use crate::{Velocity, apply_velocity};

    #[test]
    fn velocity_applied() {
        let mut world = World::default();

        world.init_resource::<Time>();
        let mut time = world.get_resource_mut::<Time>().unwrap();
        time.advance_by(Duration::from_secs(1));

        let system = world.register_system(apply_velocity);

        let obj = world
            .spawn((Transform::default(), Velocity { x: 1.0, y: 1.0 }))
            .id();

        assert_eq!(world.get::<Transform>(obj).unwrap().translation.x, 0.0);
        assert_eq!(world.get::<Transform>(obj).unwrap().translation.y, 0.0);
        let _ = world.run_system(system);
        assert_eq!(world.get::<Transform>(obj).unwrap().translation.x, 100.0);
        assert_eq!(world.get::<Transform>(obj).unwrap().translation.y, 100.0);
        let _ = world.run_system(system);
        assert_eq!(world.get::<Transform>(obj).unwrap().translation.x, 200.0);
        assert_eq!(world.get::<Transform>(obj).unwrap().translation.y, 200.0);
    }
}
