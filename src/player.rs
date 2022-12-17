use bevy::{prelude::*, sprite::Anchor};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use bevy_rapier2d::prelude::*;
use bevy_tweening::*;
use iyes_loopless::prelude::*;
use iyes_progress::prelude::AssetsLoading;
use leafwing_input_manager::prelude::*;
use std::f32;

use crate::enemy::EnemyKillEvent;
use crate::{GameState, InGameTag, MainCamera};

use super::Rotation;
use crate::leaf::{Leaf, LEAF_SIZE};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .add_plugin(TweeningPlugin)
            .add_system(component_animator_system::<Handle<Image>>)
            .add_system(component_animator_system::<Tongue>)
            // .register_inspectable::<Tongue>()
            .init_resource::<PlayerAssets>()
            .init_resource::<PlayerPos>()
            .add_event::<LandingEvent>()
            .add_enter_system(GameState::InGame, startup)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::InGame)
                    .with_system(camera_transform_system)
                    .with_system(jump_system)
                    .with_system(tongue_system)
                    .with_system(tongue_kill_system)
                    .with_system(detect_drown)
                    .into(),
            )
            .add_exit_system(
                GameState::InGame,
                |mut commands: Commands,
                 player: Query<Entity, With<Player>>,
                 camera: Query<Entity, With<MainCamera>>| {
                    commands
                        .entity(player.single())
                        .remove::<Animator<Transform>>()
                        .remove::<Animator<Handle<Image>>>();
                    commands
                        .entity(camera.single())
                        .remove::<Animator<Transform>>();
                },
            );
    }
}

fn startup(mut commands: Commands, assets: Res<PlayerAssets>, mut player_pos: ResMut<PlayerPos>) {
    *player_pos = PlayerPos::default();

    let frog = PlayerBundle {
        sprite: SpriteBundle {
            texture: assets.player[0].clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            sprite: Sprite {
                anchor: Anchor::Custom(Vec2::new(0.0, (64. - 50.) / 64.)),
                ..default()
            },
            ..default()
        },
        ..default()
    };

    let player = commands
        .spawn(frog)
        .insert(Name::new("Player"))
        .insert(InputManagerBundle::<PlayerAction> {
            action_state: ActionState::default(),
            input_map: InputMap::new([
                (KeyCode::Up, PlayerAction::Up),
                (KeyCode::W, PlayerAction::Up),
                (KeyCode::Down, PlayerAction::Down),
                (KeyCode::S, PlayerAction::Down),
                (KeyCode::Left, PlayerAction::Left),
                (KeyCode::A, PlayerAction::Left),
                (KeyCode::Right, PlayerAction::Right),
                (KeyCode::D, PlayerAction::Right),
            ]),
        })
        .insert((
            Sensor,
            Collider::cuboid(64., 64.),
            CollisionGroups::new(Group::GROUP_2, Group::GROUP_4),
            ActiveCollisionTypes::default() | ActiveCollisionTypes::STATIC_STATIC,
            // CollisionLayers::none()
            //     .with_group(CollisionLayer::Player)
            //     .with_mask(CollisionLayer::Leaf),
        ))
        .insert(InGameTag)
        .id();

    spawn_tongue(commands, player, assets);
}

#[derive(Debug, Default, Resource)]
pub struct PlayerPos(pub IVec2);

#[derive(Component, Default, Inspectable)]
pub struct Player {
    jumping: bool,
    next_pos: IVec2,
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    frog: Player,
    rotation: Rotation,
    #[bundle]
    sprite: SpriteBundle,
}

#[derive(Resource)]
pub struct PlayerAssets {
    pub player: [Handle<Image>; 3],
    tongue_base: Handle<Image>,
    tongue_tip: Handle<Image>,
    kill_sound: Handle<AudioSource>,
}

impl FromWorld for PlayerAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();

        let player = [
            assets.load("kaeru0.png"),
            assets.load("kaeru1.png"),
            assets.load("kaeru2.png"),
        ];

        let tongue_base = assets.load("tong_base.png");

        let tongue_tip = assets.load("tong_tip.png");

        let kill_sound = assets.load("syuwan.ogg");

        let mut loading = world.get_resource_mut::<AssetsLoading>().unwrap();
        player.iter().for_each(|p| loading.add(p.clone()));
        loading.add(tongue_base.clone());
        loading.add(tongue_tip.clone());
        loading.add(kill_sound.clone());

        PlayerAssets {
            player,
            tongue_base,
            tongue_tip,
            kill_sound,
        }
    }
}

struct LandingEvent;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayerAction {
    Up,
    Down,
    Left,
    Right,
}

struct TransformScalePositionLens {
    scale: lens::TransformScaleLens,
    position: lens::TransformPositionLens,
}

impl lens::Lens<Transform> for TransformScalePositionLens {
    fn lerp(&mut self, target: &mut Transform, ratio: f32) {
        self.scale.lerp(target, ratio);
        self.position.lerp(target, ratio);
    }
}

struct HandleImageLens {
    start: Handle<Image>,
    end: Handle<Image>,
}

impl lens::Lens<Handle<Image>> for HandleImageLens {
    fn lerp(&mut self, target: &mut Handle<Image>, ratio: f32) {
        if ratio < 1.0 {
            *target = self.start.clone();
        } else {
            *target = self.end.clone();
        }
    }
}

fn camera_transform_system(
    mut transform: Query<&mut Transform>,
    player: Query<Entity, With<Player>>,
    camera: Query<Entity, (With<Camera>, With<MainCamera>)>,
) {
    const CAMERA_Z: f32 = 999.9;

    let mut player_translation = transform.get(player.single()).unwrap().translation;
    player_translation.z = CAMERA_Z;

    transform.get_mut(camera.single()).unwrap().translation =
        player_translation / Vec3::new(2.0, 2.0, 1.0);
}

fn jump_system(
    mut commands: Commands,
    mut player: Query<(Entity, &mut Player, &ActionState<PlayerAction>, &Transform)>,
    mut camera: Query<Entity, (With<Camera>, With<MainCamera>)>,
    mut reader: EventReader<TweenCompleted>,
    game_assets: Res<PlayerAssets>,
    mut pos: ResMut<PlayerPos>,
    mut landing: EventWriter<LandingEvent>,
) {
    let (player_entity, mut player, player_action, player_transform) = player.single_mut();
    let camera_entity = camera.single_mut();

    for ev in reader.iter() {
        if ev.entity == player_entity {
            player.jumping = false;

            *pos = PlayerPos(player.next_pos);

            landing.send(LandingEvent);
        }
    }

    let jump = !player.jumping
        && (player_action.pressed(PlayerAction::Up)
            || player_action.pressed(PlayerAction::Down)
            || player_action.pressed(PlayerAction::Left)
            || player_action.pressed(PlayerAction::Right));

    if jump {
        // This can be local
        player.jumping = true;

        let rot;
        if player_action.pressed(PlayerAction::Up) {
            rot = Quat::from_rotation_z(0.);
            player.next_pos += IVec2::Y;
        } else if player_action.pressed(PlayerAction::Down) {
            rot = Quat::from_rotation_z(f32::consts::PI);
            player.next_pos -= IVec2::Y;
        } else if player_action.pressed(PlayerAction::Left) {
            rot = Quat::from_rotation_z(f32::consts::FRAC_PI_2);
            player.next_pos -= IVec2::X;
        } else if player_action.pressed(PlayerAction::Right) {
            rot = Quat::from_rotation_z(-f32::consts::FRAC_PI_2);
            player.next_pos += IVec2::X;
        } else {
            unreachable!();
        }

        let end = player_transform.translation + rot * Vec3::new(0., LEAF_SIZE, 0.);

        const JUMP_CAMERA_SCALE: f32 = 1.05;
        const JUMP_SCALE: f32 = 2.0;

        let tween = Tween::new(
            EaseFunction::CubicInOut,
            // TweeningType::Once,
            std::time::Duration::from_millis(200),
            lens::TransformScaleLens {
                start: Vec3::ONE,
                end: Vec2::splat(JUMP_CAMERA_SCALE).extend(1.),
            },
        )
        .then(Tween::new(
            EaseFunction::CubicInOut,
            // TweeningType::Once,
            std::time::Duration::from_millis(200),
            lens::TransformScaleLens {
                start: Vec2::splat(JUMP_CAMERA_SCALE).extend(1.),
                end: Vec3::ONE,
            },
        ));

        commands.entity(camera_entity).insert(Animator::new(tween));

        let tween = Tracks::new([
            Tween::new(
                EaseFunction::CubicInOut,
                // TweeningType::Once,
                std::time::Duration::from_millis(200),
                TransformScalePositionLens {
                    scale: lens::TransformScaleLens {
                        start: Vec3::ONE,
                        end: Vec2::splat(JUMP_SCALE).extend(1.),
                    },
                    position: lens::TransformPositionLens {
                        start: player_transform.translation,
                        end,
                    },
                },
            )
            .then(
                Tween::new(
                    EaseFunction::CubicInOut,
                    // TweeningType::Once,
                    std::time::Duration::from_millis(200),
                    lens::TransformScaleLens {
                        start: Vec2::splat(JUMP_SCALE).extend(1.),
                        end: Vec3::ONE,
                    },
                )
                .with_completed_event(0),
            ),
            Sequence::new([Tween::new(
                EaseFunction::QuadraticInOut,
                // TweeningType::Once,
                std::time::Duration::from_millis(100),
                lens::TransformRotationLens {
                    start: player_transform.rotation,
                    end: rot,
                },
            )]),
        ]);

        let image_seq = [0, 1, 2, 1, 0]
            .windows(2)
            .zip([50, 250, 50, 50])
            .map(|(idx, dur)| {
                Tween::new(
                    EaseFunction::QuadraticInOut,
                    // TweeningType::Once,
                    std::time::Duration::from_millis(dur),
                    HandleImageLens {
                        start: game_assets.player[idx[0]].clone(),
                        end: game_assets.player[idx[1]].clone(),
                    },
                )
            });
        let image_seq = Sequence::new(image_seq);

        commands
            .entity(player_entity)
            .insert(Animator::new(tween))
            .insert(Animator::new(image_seq));
    }
}

#[derive(Component, Inspectable)]
pub struct Tongue {
    length: f32,
    base: Entity,
    tip: Entity,
    extending: bool,
}

#[derive(Bundle)]
struct TongueBundle {
    name: Name,
    tongue: Tongue,
    rotation: Rotation,
    transform: Transform,
    global_transform: GlobalTransform,
    #[bundle]
    visibility: VisibilityBundle,
}

const TONGUE_LEN_DEFAULT: f32 = 32.;

impl TongueBundle {
    fn new(base: Entity, tip: Entity) -> Self {
        TongueBundle {
            name: Name::new("Tongue"),
            tongue: Tongue {
                length: TONGUE_LEN_DEFAULT,
                base,
                tip,
                extending: false,
            },
            rotation: default(),
            visibility: VisibilityBundle {
                visibility: Visibility { is_visible: false },
                computed: default(),
            },
            transform: Transform::from_translation(Vec3::new(0., 0., -0.1)),
            global_transform: default(),
        }
    }
}

fn spawn_tongue(mut commands: Commands, parent: Entity, res: Res<PlayerAssets>) {
    let base = commands
        .spawn(SpriteBundle {
            texture: res.tongue_base.clone(),
            transform: Transform::from_scale(Vec3::new(0.3, 1., 1.))
                .with_translation(Vec3::new(0., 0., 0.)),
            sprite: Sprite {
                anchor: Anchor::BottomCenter,
                ..default()
            },
            ..default()
        })
        .insert(Name::new("TongueBase"))
        .insert(InGameTag)
        .id();
    let tip = commands
        .spawn_bundle(SpriteBundle {
            texture: res.tongue_tip.clone(),
            transform: Transform::from_scale(Vec3::new(0.3, 0.3, 1.))
                .with_translation(Vec3::new(0., 64., 0.)),
            sprite: Sprite {
                anchor: Anchor::BottomCenter,
                ..default()
            },
            ..default()
        })
        .insert((
            Sensor,
            Collider::cuboid(32., 32.),
            CollisionGroups::new(Group::GROUP_3, Group::GROUP_1),
            ActiveCollisionTypes::default() | ActiveCollisionTypes::STATIC_STATIC,
            // CollisionLayers::none()
            //     .with_group(CollisionLayer::Tongue)
            //     .with_mask(CollisionLayer::Enemy),
        ))
        .insert(Name::new("TongueTip"))
        .insert(InGameTag)
        .id();

    let tongue = commands
        .spawn_bundle(TongueBundle::new(base, tip))
        .push_children(&[base, tip])
        .insert(InGameTag)
        .id();

    commands.entity(parent).add_child(tongue);
}

struct TongueLengthLens {
    start: f32,
    end: f32,
}

impl lens::Lens<Tongue> for TongueLengthLens {
    fn lerp(&mut self, target: &mut Tongue, ratio: f32) {
        target.length = self.start + (self.end - self.start) * ratio;
    }
}

fn tongue_kill_system(
    mut ev_kill: EventWriter<EnemyKillEvent>,
    tongue: Query<(&Tongue, &Visibility)>,
    rapier_ctx: Res<RapierContext>,
    game_assets: Res<PlayerAssets>,
    audio: Res<Audio>,
) {
    let (tongue, tongue_vis) = tongue.single();

    if tongue.extending && tongue_vis.is_visible {
        let mut killed = false;
        for other in get_intersections(&rapier_ctx, tongue.tip) {
            ev_kill.send(EnemyKillEvent(other));
            killed = true;
        }

        if killed {
            audio
                .play(game_assets.kill_sound.clone())
                .with_playback_rate(1.0 + (fastrand::f64() - 0.5) * 0.2);
        }
    }
}

fn tongue_system(
    mut commands: Commands,
    mut tongue: Query<(Entity, &mut Tongue, &GlobalTransform, &mut Visibility)>,
    player: Query<(Entity, &Player)>,
    mut transform: Query<&mut Transform>,
    buttons: Res<Input<MouseButton>>,
    mouse_pos: Res<super::MousePos>,
    mut reader: EventReader<TweenCompleted>,
) {
    let (tongue_entity, mut tongue, g_tr, mut visibility) = tongue.single_mut();

    let (player_entity, player) = player.single();

    transform.get_mut(tongue.base).unwrap().scale = Vec3::new(0.3, tongue.length / 128., 1.);
    transform.get_mut(tongue.tip).unwrap().translation = Vec3::new(0., tongue.length, 0.);

    for ev in reader.iter() {
        if ev.entity == tongue_entity {
            if ev.user_data == 0 {
                let tween = Tween::new(
                    EaseFunction::QuarticIn,
                    // TweeningType::Once,
                    std::time::Duration::from_millis((tongue.length as u64 / 2).max(400)),
                    TongueLengthLens {
                        start: tongue.length,
                        end: TONGUE_LEN_DEFAULT.min(tongue.length),
                    },
                )
                .with_completed_event(1);
                commands.entity(tongue_entity).insert(Animator::new(tween));

                tongue.extending = false;

                return;
            } else {
                visibility.is_visible = false;
            }
        }
    }

    if buttons.just_pressed(MouseButton::Left) && !player.jumping && !tongue.extending {
        visibility.is_visible = true;

        if let Some(mouse_pos) = mouse_pos.0 {
            let length = mouse_pos.distance(g_tr.translation().truncate()) - 32.0;

            let tween = Tween::new(
                EaseFunction::QuarticInOut,
                // TweeningType::Once,
                std::time::Duration::from_millis((length as u64 / 4).max(150)),
                TongueLengthLens {
                    start: TONGUE_LEN_DEFAULT.min(length),
                    end: length,
                },
            )
            .with_completed_event(0);
            commands.entity(tongue_entity).insert(Animator::new(tween));

            tongue.extending = true;

            let player_rot = transform.get(player_entity).unwrap().rotation;

            let to_mouse = mouse_pos - g_tr.translation().truncate();

            // player rotation will be applied, hence multiplication by the inverse of it
            transform.get_mut(tongue_entity).unwrap().rotation =
                Quat::from_rotation_z(Vec2::Y.angle_between(to_mouse)) * player_rot.inverse();
        }
    }
}

fn detect_drown(
    landing: EventReader<LandingEvent>,
    q: Query<(Entity, &Player, &Transform)>,
    leafs: Query<&Leaf>,
    rapier_ctx: Res<RapierContext>,
    mut commands: Commands,
    tran: EventReader<StateTransitionEvent<GameState>>,
) {
    let (player_entity, player, transform) = q.single();

    if player.jumping && landing.is_empty() {
        return;
    }

    let inter = get_intersections(&rapier_ctx, player_entity).collect::<Vec<_>>();

    if inter.iter().any(|&e| leafs.get(e).unwrap().decay >= 1.0) {
        commands.insert_resource(NextState(GameState::GameOver));
    } else if inter.is_empty() && transform.translation.truncate() != Vec2::new(0., 0.) {
        commands.insert_resource(NextState(GameState::GameOver));
    }
}

fn get_intersections(ctx: &RapierContext, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
    ctx.intersections_with(entity)
        .filter_map(move |(a, b, inter)| {
            if !inter {
                return None;
            }
            if a == entity {
                Some(b)
            } else {
                Some(a)
            }
        })
}
