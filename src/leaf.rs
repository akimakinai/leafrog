use std::time::Duration;

use crate::{CollisionLayer, GameState, InGameTag};

use super::Rotation;
use bevy::{prelude::*, render::render_resource::FilterMode};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_kira_audio::{Audio, AudioSource, InstanceHandle};
use heron::prelude::*;
use iyes_loopless::prelude::ConditionSet;
use iyes_progress::prelude::AssetsLoading;

pub const LEAF_SIZE: f32 = 256.0;

pub struct LeafPlugin;

impl Plugin for LeafPlugin {
    fn build(&self, app: &mut App) {
        // app.register_inspectable::<Leaf>();

        app.init_resource::<LeafAsset>().add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::InGame)
                .with_system(set_texture_filters_to_nearest)
                .with_system(leaf_decay_system)
                .with_system(leaf_rotator)
                .into(),
        );
    }
}

fn set_texture_filters_to_nearest(mut textures: ResMut<Assets<Image>>, leaf_asset: Res<LeafAsset>) {
    if let Some(mut texture) = textures.get_mut(leaf_asset.texture.clone()) {
        if texture.sampler_descriptor.mag_filter != FilterMode::Linear {
            texture.sampler_descriptor.mag_filter = FilterMode::Linear;
            texture.sampler_descriptor.min_filter = FilterMode::Linear;
        }
    }
}

pub struct LeafAsset {
    texture: Handle<Image>,
    audio_drop: Handle<AudioSource>,
}

impl FromWorld for LeafAsset {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();

        let texture = assets.load("leaf.png");
        let audio_drop = assets.load("water_drop.ogg");

        let mut loading = world.get_resource_mut::<AssetsLoading>().unwrap();
        loading.add(texture.clone());
        loading.add(audio_drop.clone());

        LeafAsset {
            texture,
            audio_drop,
        }
    }
}

pub fn spawn_leaf<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    pos: IVec2,
    asset: &LeafAsset,
) -> bevy::ecs::system::EntityCommands<'w, 's, 'a> {
    let tr = Vec2::new(pos.x as f32, pos.y as f32) * Vec2::splat(LEAF_SIZE);
    let mut e = commands.spawn_bundle(LeafBundle {
        leaf: Leaf {
            decay: 0.,
            pos,
            restore_timer: None,
        },
        sprite: SpriteBundle {
            texture: asset.texture.clone(),
            transform: Transform {
                translation: tr.extend(0.),
                ..default()
            },
            ..default()
        },
        rotation: Rotation(fastrand::f32() * (2. * std::f32::consts::PI)),
    });
    e.insert_bundle((
        RigidBody::Sensor,
        CollisionShape::Sphere { radius: 128. },
        Collisions::default(),
        CollisionLayers::all_masks::<CollisionLayer>().with_group(CollisionLayer::Leaf),
    ));
    e.insert(InGameTag);
    e
}

#[derive(Component, Inspectable)]
pub struct Leaf {
    pub decay: f32,
    pos: IVec2,
    #[inspectable(ignore)]
    restore_timer: Option<Timer>,
}

#[derive(Bundle)]
pub struct LeafBundle {
    pub leaf: Leaf,
    pub rotation: Rotation,
    #[bundle]
    pub sprite: SpriteBundle,
}

fn leaf_decay_system(
    player_pos: Res<crate::player::PlayerPos>,
    mut leaf: Query<(&mut Leaf, &mut Sprite)>,
    time: Res<Time>,
    audio: Res<Audio>,
    asset: Res<LeafAsset>,
    mut audio_handle: Local<Option<InstanceHandle>>,
) {
    let mut leaf_drop = false;

    leaf.for_each_mut(|(mut x, mut sprite)| {
        if let Some(timer) = x.restore_timer.as_mut() {
            timer.tick(time.delta());

            if timer.finished() {
                x.decay = 0.0;
                x.restore_timer = None;
            } else if timer.elapsed().as_secs_f32() / timer.duration().as_secs_f32() > 0.9 {
                x.decay = 1.0
                    - 10.0 * (timer.elapsed().as_secs_f32() / timer.duration().as_secs_f32() - 0.9);
            }
        } else {
            let pre = x.decay;

            let dd = if x.pos == player_pos.0 || x.decay > 0.7 {
                0.8
            } else {
                -0.3
            };
            x.decay = (x.decay + dd * time.delta_seconds()).clamp(0., 1.);
            if pre < 0.8 && x.decay >= 0.8 {
                leaf_drop = true;
            }
            if x.decay >= 1.0 {
                x.restore_timer = Some(Timer::new(Duration::from_secs(5), false));
            }
        }

        let g = 1.0 - 0.9 * (10. * x.decay).powi(2) / 100.;
        sprite.color = Color::rgba(g, g, g, g);
    });

    if leaf_drop {
        audio.set_playback_rate(1.0 + (fastrand::f32() - 0.5) * 0.2);
        *audio_handle = Some(audio.play(asset.audio_drop.clone()));
    }
}

fn leaf_rotator(mut q: Query<(Entity, &mut Rotation), With<Leaf>>, time: Res<Time>) {
    q.for_each_mut(|(e, mut r)| {
        let xorshift = |mut n: u32| {
            n ^= 2463534242;
            n ^= n << 13;
            n ^= n >> 17;
            n ^= n << 5;
            n
        };
        r.0 += std::f32::consts::PI
            * time.delta_seconds()
            * (xorshift(e.id()) as f32 / std::u32::MAX as f32 * 20.0)
            / 100.0;
    })
}
