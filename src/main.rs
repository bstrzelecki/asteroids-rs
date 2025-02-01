use bevy::{prelude::*, render::camera::ScalingMode};
use leafwing_input_manager::{
    plugin::InputManagerPlugin,
    prelude::{ActionState, InputMap},
    Actionlike, InputManagerBundle,
};

#[derive(Component)]
struct Player;

impl Player {
    fn default_input_mat() -> InputMap<PlayerAction> {
        let mut input_map = InputMap::default();

        use PlayerAction::*;
        input_map.insert(Forward, KeyCode::ArrowUp);
        input_map.insert(Forward, KeyCode::KeyW);

        input_map.insert(Rotate(-1), KeyCode::ArrowLeft);
        input_map.insert(Rotate(-1), KeyCode::KeyA);

        input_map.insert(Rotate(1), KeyCode::ArrowRight);
        input_map.insert(Rotate(1), KeyCode::KeyD);

        input_map
    }
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
#[derive(Actionlike, Debug, Clone, Eq, PartialEq, Hash, Reflect)]
enum PlayerAction {
    Forward,
    Rotate(i8),
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<PlayerAction>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, apply_velocity)
        .add_systems(Update, wrap_around)
        .add_systems(Update, player_input)
        .run();
}
const ACC_SPEED: f32 = 5.0;
const ROTATION_SPEED: f32 = 8.0;

const MAX_VELOCITY: f32 = 3.0;

fn player_input(
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

fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
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
        Player,
        InputManagerBundle::<PlayerAction>::with_map(Player::default_input_mat()),
    ));
    let mesh = Mesh2d(meshes.add(Circle::new(20.0)));
    cmd.spawn((
        mesh,
        MeshMaterial2d(materials.add(Color::linear_rgb(256.0, 0.0, 0.0))),
        Transform::from_xyz(50.0, 50.0, 00.0),
        Velocity { x: -3.0, y: 1.0 },
    ));
}

fn wrap_around(mut e: Query<&mut Transform>) {
    e.iter_mut().for_each(|mut it| {
        if it.translation.x < 0.0 {
            it.translation.x = 1920.0;
        }
        if it.translation.y < 0.0 {
            it.translation.y = 1080.0;
        }
        if it.translation.y > 1080.0 {
            it.translation.y = 0.0;
        }
        if it.translation.x > 1920.0 {
            it.translation.x = 0.0;
        }
    });
}

fn apply_velocity(mut e: Query<(&mut Transform, &Velocity)>) {
    e.iter_mut().for_each(|mut it| {
        it.0.translation.x += it.1.x;
        it.0.translation.y += it.1.y;
    });
}
