use crate::{InGameTag, Rotation};
use bevy::prelude::*;
use iyes_progress::prelude::AssetsLoading;
use bevy_rapier2d::prelude::*;
use std::f32;

use super::GameState;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyAssets>()
            .add_event::<EnemyKillEvent>()
            .add_system(spawn_bugs.in_schedule(OnEnter(GameState::InGame)))
            .add_systems(
                (enemy_move_system, enemy_reset).in_set(OnUpdate(GameState::InGame))
            );
    }
}

#[derive(Resource)]
struct EnemyAssets {
    texture: Handle<Image>,
}

impl FromWorld for EnemyAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();

        let texture = assets.load("bug.png");

        let mut loading = world.get_resource_mut::<AssetsLoading>().unwrap();
        loading.add(texture.clone());

        EnemyAssets { texture }
    }
}

fn enemy_move_system(mut q: Query<(&mut Transform, &mut Rotation, &Velocity)>, time: Res<Time>) {
    q.for_each_mut(|(mut tr, mut rot, vel)| {
        let rotation = tr.rotation;
        tr.translation += rotation * Vec3::Y * vel.0 * time.delta_seconds();

        if tr.translation.distance(Vec3::ZERO) > 700. {
            rot.0 += 2.0 * time.delta_seconds();
        }
    });
}

fn random_initial_pos_rot() -> (Vec2, Rotation) {
    let random_angle = || fastrand::f32() * (2. * f32::consts::PI);
    let pos = Quat::from_rotation_z(random_angle())
        .mul_vec3(Vec3::Y * 600.)
        .truncate();
    (pos, Rotation(random_angle()))
}

fn spawn_bugs(mut commands: Commands, assets: Res<EnemyAssets>) {
    for _ in 0..20 {
        let (pos, rot) = random_initial_pos_rot();
        commands
            .spawn(
                (
                    BugBundle::new(assets.texture.clone(), pos, rot),
                    Sensor,
                    Collider::cuboid(20., 20.),
                    CollisionGroups::new(Group::GROUP_1, Group::ALL),
                    ActiveCollisionTypes::default() | ActiveCollisionTypes::STATIC_STATIC,
            )
            )
            .insert(InGameTag);
    }
}

#[derive(Component)]
struct Velocity(f32);

#[derive(Component, Default)]
struct Bug;

#[derive(Bundle)]
struct BugBundle {
    bug: Bug,
    rotation: Rotation,
    velocity: Velocity,
    #[bundle]
    sprite: SpriteBundle,
}

impl BugBundle {
    fn new(res: Handle<Image>, translation: Vec2, rotation: Rotation) -> Self {
        BugBundle {
            bug: default(),
            rotation,
            velocity: Velocity(300.0),
            sprite: SpriteBundle {
                texture: res.clone(),
                transform: Transform {
                    scale: Vec2::splat(0.8).extend(1.0),
                    translation: translation.extend(3.0),
                    rotation: default(),
                },
                ..default()
            },
        }
    }
}

pub struct EnemyKillEvent(pub Entity);

fn enemy_reset(
    mut ev_kill: EventReader<EnemyKillEvent>,
    mut q: Query<(&mut Transform, &mut Rotation), With<Bug>>,
) {
    for ev in ev_kill.iter() {
        let (mut transform, mut rot) = q.get_mut(ev.0).unwrap();
        let (new_pos, new_rot) = random_initial_pos_rot();
        *rot = new_rot;
        transform.translation = new_pos.extend(3.0);
    }
}
