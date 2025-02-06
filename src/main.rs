use bevy::{prelude::*, render::camera::ScalingMode};
use leafwing_input_manager::{plugin::InputManagerPlugin, InputManagerBundle};
use player::Player;

mod player;

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
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<player::PlayerAction>::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                apply_velocity,
                wrap_around,
                player::player_input,
                player::shoot_projectile,
            ),
        )
        .run();
}
const ACC_SPEED: f32 = 5.0;
const ROTATION_SPEED: f32 = 8.0;

const MAX_VELOCITY: f32 = 3.0;
const SHOOT_TIMEOUT: f32 = 0.5;

const PROJECTILE_SPEED: f32 = 10.0;

#[derive(Resource)]
struct ProjectileSprite(Handle<ColorMaterial>, Handle<Mesh>);

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
                width: 1920.0,
                height: 1080.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(1920.0 / 2.0, 1080.0 / 2.0, 0.0),
    ));

    let player_mesh = Mesh2d(meshes.add(Triangle2d::new(
        Vec2::new(0.0, 50.0),
        Vec2::new(-50.0, -50.0),
        Vec2::new(50.0, -50.0),
    )));
    cmd.spawn((
        player_mesh,
        MeshMaterial2d(materials.add(Color::linear_rgb(256.0, 0.0, 0.0))),
        Transform::from_xyz(150.0, 50.0, 00.0),
        Velocity { x: 0.0, y: 0.0 },
        Player::default(),
        InputManagerBundle::<player::PlayerAction>::with_map(Player::default_input_map()),
    ));
    let mesh = Mesh2d(meshes.add(Circle::new(20.0)));
    cmd.spawn((
        mesh,
        MeshMaterial2d(materials.add(Color::linear_rgb(256.0, 0.0, 0.0))),
        Transform::from_xyz(50.0, 50.0, 00.0),
        Velocity { x: -3.0, y: 1.0 },
    ));
}

#[derive(Component)]
struct WrapTimeout(u8);

fn wrap_around(
    mut e: Query<(Entity, &mut Transform, Option<&mut WrapTimeout>)>,
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
            if timeout.0 == 0 {
                cmd.entity(e).despawn();
                return;
            }
            timeout.0 -= 1;
        }
    });
}

fn apply_velocity(mut e: Query<(&mut Transform, &Velocity)>) {
    e.iter_mut().for_each(|mut it| {
        it.0.translation.x += it.1.x;
        it.0.translation.y += it.1.y;
    });
}
