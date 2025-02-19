use bevy::app::{App, Plugin, Startup};
use bevy::prelude::*;
use bevy_hanabi::{
    Attribute, ColorOverLifetimeModifier, EffectAsset, ExprWriter, Gradient, HanabiPlugin,
    ParticleEffect, ParticleEffectBundle, ScalarType, SetAttributeModifier,
    SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension, Spawner,
};

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, cleanup_after_timeout)
            .init_resource::<CollisionEffect>()
            .add_plugins(HanabiPlugin);
    }
}
#[derive(Resource, Default)]
pub struct CollisionEffect(pub Handle<EffectAsset>);

#[derive(Component)]
pub struct CleanupAfterTimeout(Timer);

impl Default for CleanupAfterTimeout {
    fn default() -> Self {
        Self(Timer::from_seconds(3.0, TimerMode::Once))
    }
}

fn cleanup_after_timeout(
    mut cmd: Commands,
    time: Res<Time>,
    mut timer: Query<(&mut CleanupAfterTimeout, Entity)>,
) {
    timer.iter_mut().for_each(|(mut timer, e)| {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            cmd.entity(e).despawn();
            timer.0.reset();
        }
    });
}

fn setup(
    mut effect: ResMut<CollisionEffect>,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut cmd: Commands,
) {
    let writer = ExprWriter::new();

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(1.).expr(),
        dimension: ShapeDimension::Surface,
    };

    let vel_skew = writer.add_property("vel_skew", 20.0.into());

    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: (writer.rand(ScalarType::Float) * writer.lit(140.)
            + writer.lit(20.)
            + writer.prop(vel_skew))
        .expr(),
    };

    let lifetime = writer.lit(3.).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(1., 0., 0., 1.));
    gradient.add_key(1.0, Vec4::splat(0.));

    let effect_handle = effects.add(
        EffectAsset::new(2048, Spawner::once(2048.0.into(), true), writer.finish())
            .init(init_pos)
            .init(init_vel)
            .init(init_lifetime)
            .render(ColorOverLifetimeModifier { gradient }),
    );
    effect.0 = effect_handle;
    cmd.spawn((
        // First effect doesn't load (#319)
        ParticleEffectBundle {
            effect: ParticleEffect::new(effect.0.clone()),
            transform: Transform::from_xyz(-100.0, -100.0, 0.0),
            ..default()
        },
        CleanupAfterTimeout::default(),
    ));
}
