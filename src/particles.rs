use bevy::app::{App, Plugin, Startup, Update};
use bevy::prelude::*;
use bevy_hanabi::{
    Attribute, ColorOverLifetimeModifier, EffectAsset, EffectProperties, ExprWriter, Gradient,
    HanabiPlugin, ParticleEffect, ParticleEffectBundle, ScalarType, SetAttributeModifier,
    SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension, Spawner,
};

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (player_input, apply_shadow, shoot_projectile))
            .init_resource::<CollisionEffect>()
            .add_plugins(HanabiPlugin);
    }
}
#[derive(Resource, Default)]
pub struct CollisionEffect(pub Handle<EffectAsset>);

fn setup(mut effect: ResMut<CollisionEffect>, mut effects: ResMut<Assets<EffectAsset>>) {
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
}
