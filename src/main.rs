use std::time::Duration;

use asteroid::AsteroidPlugin;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_rand::plugin::EntropyPlugin;
use bevy_spatial::kdtree::KDTree2;
use bevy_spatial::{AutomaticUpdate, SpatialAccess, SpatialStructure, TransformMode};
use leafwing_input_manager::plugin::InputManagerPlugin;
use particles::ParticlePlugin;
use player::{OnPlayerDamage, PlayerPlugin};

mod asteroid;
mod particles;
mod player;

type RngType = bevy_prng::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            InputManagerPlugin::<player::PlayerAction>::default(),
            EntropyPlugin::<RngType>::default(),
            AutomaticUpdate::<SpatialMarker>::new()
                .with_frequency(Duration::from_millis(16))
                .with_spatial_ds(SpatialStructure::KDTree2)
                .with_transform(TransformMode::GlobalTransform),
        ))
        .add_plugins((PlayerPlugin, ParticlePlugin, AsteroidPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                apply_velocity,
                wrap_around,
                check_collisions,
                check_for_gameover,
            ),
        )
        .add_event::<CollisionEvent>()
        .add_observer(update_score)
        .add_observer(update_lives)
        .run();
}

const ACC_SPEED: f32 = 5.0;
const ROTATION_SPEED: f32 = 8.0;
const MAX_VELOCITY: f32 = 3.0;

const SHOOT_TIMEOUT: f32 = 0.5;
const PROJECTILE_SPEED: f32 = 10.0;

const WINDOW_WIDTH: f32 = 1920.0;
const WINDOW_HEIGHT: f32 = 1080.0;

const SMALL_ASTEROID_RADIUS: f32 = 20.0;
const LARGE_ASTEROID_RADIUS: f32 = 40.0;

#[derive(Resource)]
struct ProjectileSprite(Handle<ColorMaterial>, Handle<Mesh>);

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

fn update_score(event: Trigger<OnScoreUpdate>, mut text: Query<(&mut Text, &mut Score)>) {
    text.iter_mut().for_each(|(mut text, mut score)| {
        score.0 += event.0;
        text.0 = format!("Score: {}", score.0);
    });
}

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
    cmd.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Start,
        justify_content: JustifyContent::Center,
        padding: UiRect {
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(10.0),
            bottom: Val::Px(0.0),
        },
        ..default()
    })
    .with_child((Text::new("Score: 0"), Score::default()));
    cmd.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Start,
        justify_content: JustifyContent::Start,
        padding: UiRect {
            left: Val::Px(10.0),
            right: Val::Px(0.0),
            top: Val::Px(10.0),
            bottom: Val::Px(0.0),
        },
        ..default()
    })
    .with_child((Text::new("X X X"), Lives::default()));
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

fn check_for_gameover(lives: Option<Single<&Lives, Changed<Lives>>>) {
    if let Some(lives) = lives {
        if lives.0 <= 0 {
            println!("Player dead!");
        }
    }
}

fn update_lives(_event: Trigger<OnPlayerDamage>, mut text: Query<(&mut Text, &mut Lives)>) {
    text.iter_mut().for_each(|(mut text, mut lives)| {
        lives.0 -= 1;
        text.0 = "X ".repeat(lives.0 as usize);
    });
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
