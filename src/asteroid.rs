use std::time::Duration;

use bevy::prelude::*;
use bevy_hanabi::{ParticleEffect, ParticleEffectBundle};
use bevy_rand::{global::GlobalEntropy, prelude::Entropy, traits::ForkableRng};
use lightyear::prelude::is_server;
use lightyear::prelude::server::Replicate;
use rand::prelude::Rng;
use rand_distr::Distribution;
use serde::{Deserialize, Serialize};

use crate::{
    CircleCollider, CleanupOnGameOver, CollisionEvent, GameState, LARGE_ASTEROID_RADIUS, RngType,
    SMALL_ASTEROID_RADIUS, Velocity, WINDOW_HEIGHT, WINDOW_WIDTH, WrapTimeout,
    particles::CleanupAfterTimeout, player::ScoreMarker,
};

pub struct AsteroidPlugin;

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    spawn_asteroid.run_if(is_server),
                    handle_grace,
                    resolve_asteroid_collisions,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_observer(divide_on_collision);
    }
}

#[derive(Component)]
pub struct AsteroidSpawner {
    timer: Timer,
    material: Handle<ColorMaterial>,
    small_mesh: Handle<Mesh>,
    large_mesh: Handle<Mesh>,
}

fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut global: GlobalEntropy<RngType>,
) {
    let small_asteroid_mesh = meshes.add(Circle::new(SMALL_ASTEROID_RADIUS));
    let large_asteroid_mesh = meshes.add(Circle::new(LARGE_ASTEROID_RADIUS));
    let asteroid_mat = materials.add(Color::linear_rgb(256.0, 0.0, 0.0));
    cmd.spawn((
        AsteroidSpawner::new(small_asteroid_mesh, large_asteroid_mesh, asteroid_mat),
        global.fork_rng(),
    ));
}

#[derive(Component, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct LargeAsteroid;

impl AsteroidSpawner {
    fn new(
        small_mesh: Handle<Mesh>,
        large_mesh: Handle<Mesh>,
        material: Handle<ColorMaterial>,
    ) -> Self {
        Self {
            timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
            small_mesh,
            large_mesh,
            material,
        }
    }

    fn asteroid(
        &self,
        pos: Transform,
        is_large: bool,
        velocity: Velocity,
        grace: bool,
    ) -> impl Bundle {
        (
            Transform::from_translation(pos.translation),
            velocity,
            self.asteroid_client(is_large),
            WrapTimeout(5),
            if grace {
                PostSpawnGrace::default()
            } else {
                Default::default()
            },
            CleanupOnGameOver,
            Replicate::default(),
        )
    }

    pub fn asteroid_client(&self, is_large: bool) -> impl Bundle {
        (
            Mesh2d(if is_large {
                self.large_mesh.clone()
            } else {
                self.small_mesh.clone()
            }),
            MeshMaterial2d(self.material.clone()),
        )
    }

    fn velocity(&self, rng: &mut Entropy<RngType>) -> Velocity {
        let velocity = rand_distr::Uniform::new(-3.0, 3.0);
        Velocity {
            x: velocity.sample(&mut *rng),
            y: velocity.sample(&mut *rng),
        }
    }

    fn spawn(&self, cmd: &mut Commands, rng: &mut Entropy<RngType>) {
        let screen_distr_x = rand_distr::Uniform::new(0.0, WINDOW_WIDTH);
        let screen_distr_y = rand_distr::Uniform::new(0.0, WINDOW_HEIGHT);
        let axis = rng.gen_bool(0.5);
        let is_large = rng.gen_bool(0.2);
        let mut asteroid = cmd.spawn(self.asteroid(
            Transform::from_xyz(
                if axis {
                    screen_distr_x.sample(&mut *rng)
                } else {
                    0.0
                },
                if !axis {
                    screen_distr_y.sample(&mut *rng)
                } else {
                    0.0
                },
                0.0,
            ),
            is_large,
            self.velocity(&mut *rng),
            false,
        ));
        if is_large {
            asteroid.insert(LargeAsteroid);
        }
        asteroid.insert(CircleCollider::new(if is_large {
            LARGE_ASTEROID_RADIUS
        } else {
            SMALL_ASTEROID_RADIUS
        }));
    }
}

#[derive(Event)]
struct Divide(Transform);

#[derive(Component)]
struct PostSpawnGrace {
    timer: Timer,
    collider_radious: f32,
}

impl Default for PostSpawnGrace {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
            collider_radious: 20.0,
        }
    }
}

fn handle_grace(mut e: Query<(Entity, &mut PostSpawnGrace)>, mut cmd: Commands, time: Res<Time>) {
    e.iter_mut().for_each(|(e, mut grace)| {
        grace.timer.tick(time.delta());
        if grace.timer.finished() {
            cmd.entity(e).remove::<PostSpawnGrace>();
            cmd.entity(e)
                .insert(CircleCollider::new(grace.collider_radious));
        }
    });
}

fn divide_on_collision(
    trigger: Trigger<Divide>,
    mut cmd: Commands,
    mut spawner: Query<(&AsteroidSpawner, &mut Entropy<RngType>)>,
) {
    let (spawner, mut rng) = spawner.single_mut();
    cmd.spawn(spawner.asteroid(trigger.0, false, spawner.velocity(&mut rng), true));
    cmd.spawn(spawner.asteroid(trigger.0, false, spawner.velocity(&mut rng), true));
}

fn spawn_asteroid(
    mut cmd: Commands,
    time: Res<Time>,
    mut spawner: Query<(&mut AsteroidSpawner, &mut Entropy<RngType>)>,
) {
    let (mut spawner, mut rng) = spawner.single_mut();
    spawner.timer.tick(time.delta());

    if spawner.timer.finished() {
        spawner.spawn(&mut cmd, &mut rng);
        spawner.timer.reset();
    }
}

fn resolve_asteroid_collisions(
    mut e: EventReader<CollisionEvent>,
    mut cmd: Commands,
    asteroids: Query<(&WrapTimeout, &Transform, Option<&LargeAsteroid>), Without<ScoreMarker>>,
    effect: Res<crate::particles::CollisionEffect>,
) {
    for ev in e.read() {
        if let Ok((_, transform, is_large)) = asteroids.get(ev.0) {
            cmd.entity(ev.0).try_despawn();
            cmd.spawn((
                ParticleEffectBundle {
                    effect: ParticleEffect::new(effect.0.clone()),
                    transform: *transform,
                    ..default()
                },
                CleanupAfterTimeout::default(),
            ));
            if is_large.is_some() {
                cmd.trigger(Divide(*transform));
            }
        }
        if asteroids.get(ev.1).is_ok() {
            cmd.entity(ev.1).try_despawn();
        }
    }
}
