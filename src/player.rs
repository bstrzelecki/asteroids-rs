use bevy::{prelude::*, time::Timer};
use leafwing_input_manager::{
    Actionlike, InputManagerBundle,
    prelude::{ActionState, InputMap},
};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    ACC_SPEED, CircleCollider, CollisionEvent, MAX_VELOCITY, OnScoreUpdate, PROJECTILE_SPEED,
    ProjectileSprite, ROTATION_SPEED, SHOOT_TIMEOUT, Velocity, WINDOW_HEIGHT, WINDOW_WIDTH,
    WrapTimeout,
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

#[derive(Actionlike, Debug, Clone, Eq, PartialEq, Hash, Reflect)]
pub enum PlayerAction {
    Forward,
    Shoot,
    Rotate(i8),
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                player_input,
                apply_shadow,
                shoot_projectile,
                resolve_bullet_collisions,
            ),
        );
    }
}
fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let player_mesh = Mesh2d(meshes.add(Triangle2d::new(
        Vec2::new(0.0, 50.0),
        Vec2::new(-50.0, -50.0),
        Vec2::new(50.0, -50.0),
    )));
    cmd.spawn((
        player_mesh.clone(),
        MeshMaterial2d(materials.add(Color::linear_rgb(256.0, 0.0, 0.0))),
        Transform::from_xyz(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0, 0.0),
        Velocity { x: 0.0, y: 0.0 },
        Player::default(),
        InputManagerBundle::<PlayerAction>::with_map(Player::default_input_map()),
    ));
    for shadow in PlayerShadow::iter() {
        cmd.spawn((
            player_mesh.clone(),
            MeshMaterial2d(materials.add(Color::linear_rgb(256.0, 0.0, 0.0))),
            Transform::from_xyz(0.0, 0.0, 0.0),
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

#[derive(Component)]
struct ScoreMarker;

pub fn shoot_projectile(
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
    let player = player.into_inner();
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

fn resolve_bullet_collisions(
    mut e: EventReader<CollisionEvent>,
    mut cmd: Commands,
    asteroids: Query<(
        &WrapTimeout,
        &Transform,
        Option<&crate::asteroid::LargeAsteroid>,
    )>,
    bullet: Query<&ScoreMarker>,
) {
    for ev in e.read() {
        if let Ok((_, _, is_large)) = asteroids.get(ev.0) {
            if bullet.get(ev.1).is_ok() {
                dbg!("bullet hit asteroid");
                if is_large.is_some() {
                    cmd.trigger(OnScoreUpdate(25));
                } else {
                    cmd.trigger(OnScoreUpdate(10));
                }
            }
        }
        if let Ok((_, _, is_large)) = asteroids.get(ev.1) {
            if bullet.get(ev.0).is_ok() {
                dbg!("bullet hit asteroid");
                if is_large.is_some() {
                    cmd.trigger(OnScoreUpdate(25));
                } else {
                    cmd.trigger(OnScoreUpdate(10));
                }
            }
        }
    }
}
