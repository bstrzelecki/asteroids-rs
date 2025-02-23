use bevy::{prelude::*, time::Timer};
use client::InputManager;
use leafwing_input_manager::{
    Actionlike, InputManagerBundle,
    prelude::{ActionState, InputMap},
};
use lightyear::{client::input::native::InputSystemSet, prelude::*};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    ACC_SPEED, CircleCollider, CleanupOnGameOver, CollisionEvent, GameState, MAX_VELOCITY,
    OnScoreUpdate, PROJECTILE_SPEED, ROTATION_SPEED, SHOOT_TIMEOUT, Velocity, WINDOW_HEIGHT,
    WINDOW_WIDTH, WrapTimeout,
};

pub struct PlayerPlugin;

#[derive(Component)]
pub struct Player {
    projectile_spawn_delay: Timer,
}

#[derive(Component, EnumIter)]
pub enum PlayerShadow {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Actionlike, Debug, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub enum PlayerAction {
    Forward,
    Shoot,
    Rotate(i8),
    None,
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                OnEnter(GameState::Playing),
                (game_setup, host_setup.run_if(is_server)).chain(),
            )
            .add_systems(
                FixedPreUpdate,
                input_passthrough.in_set(InputSystemSet::BufferInputs),
            )
            .add_systems(
                Update,
                (
                    player_input.run_if(is_server),
                    apply_shadow,
                    shoot_projectile,
                    resolve_bullet_collisions,
                    resolve_player_collisions,
                    clear_player_grace,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_observer(player_grace);
    }
}

#[derive(Resource)]
pub struct ProjectileSprite(pub Handle<ColorMaterial>, pub Handle<Mesh>);

#[derive(Component, PartialEq, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

#[derive(Component)]
pub struct PlayerSpawner {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

impl PlayerSpawner {
    fn new(mesh: Handle<Mesh>, material: Handle<ColorMaterial>) -> Self {
        Self { mesh, material }
    }

    pub fn player_client(&self) -> impl Bundle {
        (
            Mesh2d(self.mesh.clone()),
            MeshMaterial2d(self.material.clone()),
        )
    }
}

fn setup(
    mut cmd: Commands,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    cmd.insert_resource(ProjectileSprite(
        materials.add(Color::linear_rgb(0.0, 256.0, 0.0)),
        meshes.add(Circle::new(20.0)),
    ));
    let player_mesh = meshes.add(Triangle2d::new(
        Vec2::new(0.0, 50.0),
        Vec2::new(-50.0, -50.0),
        Vec2::new(50.0, -50.0),
    ));
    let mat = materials.add(Color::linear_rgb(256.0, 0.0, 0.0));
    cmd.spawn(PlayerSpawner::new(player_mesh.clone(), mat.clone()));
}
fn host_setup(mut cmd: Commands, spawner: Single<&PlayerSpawner>, e: Single<Entity, With<Player>>) {
    cmd.entity(*e).insert(spawner.player_client());
}

fn game_setup(mut cmd: Commands, spawner: Single<&PlayerSpawner>) {
    cmd.spawn((
        //spawner.player_client(),
        Transform::from_xyz(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0, 0.0),
        Velocity { x: 0.0, y: 0.0 },
        Player::default(),
        InputManagerBundle::<PlayerAction>::with_map(Player::default_input_map()),
        CircleCollider::new(15.0),
        CleanupOnGameOver,
        PlayerId(0),
        server::Replicate::default(),
    ));
    for shadow in PlayerShadow::iter() {
        cmd.spawn((
            spawner.player_client(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            CleanupOnGameOver,
            shadow,
        ));
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            projectile_spawn_delay: Timer::from_seconds(SHOOT_TIMEOUT, TimerMode::Once),
        }
    }
}

impl Player {
    pub fn default_input_map() -> InputMap<PlayerAction> {
        let mut input_map = InputMap::default();

        use PlayerAction::*;
        input_map.insert(Forward, KeyCode::ArrowUp);
        input_map.insert(Forward, KeyCode::KeyW);

        input_map.insert(Rotate(-1), KeyCode::ArrowLeft);
        input_map.insert(Rotate(-1), KeyCode::KeyA);

        input_map.insert(Rotate(1), KeyCode::ArrowRight);
        input_map.insert(Rotate(1), KeyCode::KeyD);

        input_map.insert(Shoot, KeyCode::Space);

        input_map
    }
}

fn input_passthrough(
    tick_manager: Res<TickManager>,
    mut input_manager: ResMut<InputManager<PlayerAction>>,
    player: Option<Single<&ActionState<PlayerAction>, With<Player>>>,
) {
    if player.is_none() {
        return;
    }
    let player = player.unwrap();
    let tick = tick_manager.tick();
    let mut some = false;
    for key in player.get_pressed() {
        input_manager.add_input(key, tick);
        some = true;
    }
    if !some {
        input_manager.add_input(PlayerAction::None, tick);
    }
}

pub fn player_input(
    player: Single<(&mut Velocity, &mut Transform, &ActionState<PlayerAction>), With<Player>>,
    time: Res<Time>,
) {
    let (mut velocity, mut transform, action_state) = player.into_inner();
    let direction = transform.rotation * Vec3::Y;
    let translation = direction * ACC_SPEED * time.delta().as_secs_f32();

    if action_state.pressed(&PlayerAction::Forward) {
        velocity.update(translation.xy());
    }
    velocity.max(MAX_VELOCITY);

    if action_state.pressed(&PlayerAction::Rotate(-1)) {
        transform.rotate_z(ROTATION_SPEED * time.delta_secs());
    }
    if action_state.pressed(&PlayerAction::Rotate(1)) {
        transform.rotate_z(-ROTATION_SPEED * time.delta_secs());
    }
}

#[derive(Component, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ScoreMarker;

fn shoot_projectile(
    player: Single<(
        &Transform,
        &Velocity,
        &ActionState<PlayerAction>,
        &mut Player,
    )>,
    mut cmd: Commands,
    time: Res<Time>,
    material: Option<Res<ProjectileSprite>>,
) {
    if let Some(material) = material {
        let (player, velocity, action_state, mut timer) = player.into_inner();
        timer.projectile_spawn_delay.tick(time.delta());

        if action_state.just_pressed(&PlayerAction::Shoot)
            && timer.projectile_spawn_delay.finished()
        {
            let direction = player.rotation * Vec3::Y;
            cmd.spawn((
                Mesh2d(material.1.clone()),
                Transform::from_translation(player.translation),
                MeshMaterial2d(material.0.clone()),
                Velocity {
                    x: velocity.x + direction.x * PROJECTILE_SPEED,
                    y: velocity.y + direction.y * PROJECTILE_SPEED,
                },
                WrapTimeout(1),
                CircleCollider::new(10.0),
                ScoreMarker,
                CleanupOnGameOver,
            ));
            timer.projectile_spawn_delay.reset();
        }
    } else {
        warn!("Projectile material not loaded");
    }
}

pub fn apply_shadow(
    player: Single<&Transform, With<Player>>,
    mut shadows: Query<(&mut Transform, &PlayerShadow), Without<Player>>,
) {
    shadows.iter_mut().for_each(|(mut it, shadow)| {
        use PlayerShadow::*;
        it.translation = player.translation
            + Vec3::new(
                match shadow {
                    Left => -WINDOW_WIDTH,
                    Right => WINDOW_WIDTH,
                    _ => 0.0,
                },
                match shadow {
                    Bottom => WINDOW_HEIGHT,
                    Top => -WINDOW_HEIGHT,
                    _ => 0.0,
                },
                0.0,
            );
        it.rotation = player.rotation;
    });
}

#[derive(Event)]
pub struct OnPlayerDamage;

#[derive(Component)]
struct PlayerGrace {
    timer: Timer,
}

impl Default for PlayerGrace {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        }
    }
}

fn resolve_player_collisions(
    mut e: EventReader<CollisionEvent>,
    mut cmd: Commands,
    player: Query<Entity, (With<Player>, Without<PlayerGrace>)>,
    bullets: Query<Entity, With<ScoreMarker>>,
) {
    for ev in e.read() {
        if (player.get(ev.0).is_ok() && bullets.get(ev.1).is_err())
            || (player.get(ev.1).is_ok() && bullets.get(ev.0).is_err())
        {
            if cmd.get_entity(ev.0).is_none() || cmd.get_entity(ev.1).is_none() {
                continue;
            }
            cmd.trigger(OnPlayerDamage);
        }
    }
}

fn clear_player_grace(
    mut e: Query<(Entity, &mut PlayerGrace)>,
    mut cmd: Commands,
    time: Res<Time>,
) {
    e.iter_mut().for_each(|(e, mut grace)| {
        grace.timer.tick(time.delta());
        if grace.timer.finished() {
            cmd.entity(e).remove::<PlayerGrace>();
        }
    });
}

fn player_grace(
    _event: Trigger<OnPlayerDamage>,
    mut cmd: Commands,
    player: Query<Entity, With<Player>>,
) {
    cmd.entity(player.single()).insert(PlayerGrace::default());
}

fn resolve_bullet_collisions(
    mut e: EventReader<CollisionEvent>,
    mut cmd: Commands,
    asteroids: Query<(
        &WrapTimeout,
        &Transform,
        Option<&crate::asteroid::LargeAsteroid>,
    )>,
    bullet: Query<(Entity, &ScoreMarker)>,
) {
    for ev in e.read() {
        if let Ok((_, _, is_large)) = asteroids.get(ev.0) {
            if bullet.get(ev.1).is_ok() {
                if is_large.is_some() {
                    cmd.trigger(OnScoreUpdate(25));
                } else {
                    cmd.trigger(OnScoreUpdate(10));
                }
                cmd.entity(bullet.get(ev.1).unwrap().0).despawn();
            }
        }
        if let Ok((_, _, is_large)) = asteroids.get(ev.1) {
            if bullet.get(ev.0).is_ok() {
                if is_large.is_some() {
                    cmd.trigger(OnScoreUpdate(25));
                } else {
                    cmd.trigger(OnScoreUpdate(10));
                }
                cmd.entity(bullet.get(ev.0).unwrap().0).despawn();
            }
        }
    }
}
