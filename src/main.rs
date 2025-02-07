use std::time::Duration;

use bevy::{prelude::*, render::camera::ScalingMode};
use leafwing_input_manager::plugin::InputManagerPlugin;
use player::PlayerPlugin;

mod player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<player::PlayerAction>::default())
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (apply_velocity, wrap_around, spawn_asteroid))
        .run();
}

const ACC_SPEED: f32 = 5.0;
const ROTATION_SPEED: f32 = 8.0;
const MAX_VELOCITY: f32 = 3.0;

const SHOOT_TIMEOUT: f32 = 0.5;
const PROJECTILE_SPEED: f32 = 10.0;

const WINDOW_WIDTH: f32 = 1920.0;
const WINDOW_HEIGHT: f32 = 1080.0;

#[derive(Resource)]
struct ProjectileSprite(Handle<ColorMaterial>, Handle<Mesh>);

#[derive(Component)]
struct AsteroidSpawner {
    timer: Timer,
    material: Handle<ColorMaterial>,
    mesh: Handle<Mesh>,
}

impl AsteroidSpawner {
    fn new(mesh: Handle<Mesh>, material: Handle<ColorMaterial>) -> Self {
        Self {
            timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
            mesh,
            material,
        }
    }
    fn spawn(&self, cmd: &mut Commands) {
        cmd.spawn((
            Transform::from_xyz(1920.0, 1080.0, 0.0),
            Velocity {
                x: 2.0 - 1.0,
                y: 1.0,
            },
            Mesh2d(self.mesh.clone()),
            MeshMaterial2d(self.material.clone()),
            WrapTimeout(5),
        ));
    }
}

fn spawn_asteroid(mut cmd: Commands, time: Res<Time>, mut spawner: Query<(&mut AsteroidSpawner)>) {
    let (mut spawner) = spawner.single_mut();
    spawner.timer.tick(time.delta());

    if spawner.timer.finished() {
        spawner.spawn(&mut cmd);
        spawner.timer.reset();
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

    let asteroid_mesh = meshes.add(Circle::new(20.0));
    let asteroid_mat = materials.add(Color::linear_rgb(256.0, 0.0, 0.0));
    cmd.spawn((AsteroidSpawner::new(asteroid_mesh, asteroid_mat),));
}

#[derive(Component)]
struct WrapTimeout(u8);

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

fn apply_velocity(mut e: Query<(&mut Transform, &Velocity)>) {
    e.iter_mut().for_each(|mut it| {
        it.0.translation.x += it.1.x;
        it.0.translation.y += it.1.y;
    });
}
