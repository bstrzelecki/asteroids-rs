use bevy::{prelude::*, time::Timer};
use leafwing_input_manager::{
    prelude::{ActionState, InputMap},
    Actionlike,
};

use crate::{
    ProjectileSprite, Velocity, WrapTimeout, ACC_SPEED, MAX_VELOCITY, PROJECTILE_SPEED,
    ROTATION_SPEED, SHOOT_TIMEOUT,
};

#[derive(Component)]
pub struct Player {
    projectile_spawn_delay: Timer,
}

#[derive(Actionlike, Debug, Clone, Eq, PartialEq, Hash, Reflect)]
pub enum PlayerAction {
    Forward,
    Shoot,
    Rotate(i8),
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
    mut player: Query<(&mut Velocity, &mut Transform, &ActionState<PlayerAction>), With<Player>>,
    time: Res<Time>,
) {
    let (mut velocity, mut transform, action_state) = player.single_mut();
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

pub fn shoot_projectile(
    mut player: Query<(
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
        let (player, velocity, action_state, mut timer) = player.single_mut();
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
            ));
            timer.projectile_spawn_delay.reset();
        }
    } else {
        warn!("Projectile material not loaded");
    }
}
