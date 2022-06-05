#![allow(clippy::forget_non_drop)]

use bevy::prelude::*;
use bevy_inspector_egui::{Inspectable, RegisterInspectable, WorldInspectorPlugin};
use bevy_kira_audio::{AudioApp, AudioChannel, AudioPlugin, AudioSource};
use enemy::EnemyKillEvent;
use heron::prelude::*;
use iyes_loopless::prelude::*;
use iyes_progress::{prelude::AssetsLoading, ProgressPlugin};
use std::f32;

pub mod enemy;
mod gameover;
mod leaf;
mod player;
mod state_transition;
mod title;

use leaf::LeafAsset;

fn main() {
    App::new().add_plugin(GamePlugin).run();
}

struct BGMTrack;

struct GamePlugin;

#[derive(Component)]
struct InGameTag;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum GameState {
    AssetLoading,
    Title,
    InGame,
    GameOver,
}

#[derive(PhysicsLayer)]
enum CollisionLayer {
    Enemy,
    Tongue,
    Player,
    Leaf,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::CYAN))
            .add_plugins(DefaultPlugins)
            // .add_plugin(WorldInspectorPlugin::new())
            .add_plugin(AudioPlugin)
            .add_plugin(PhysicsPlugin::default())
            // .add_plugin(heron_debug::DebugPlugin::default())
            .add_loopless_state(GameState::AssetLoading)
            .add_plugin(
                ProgressPlugin::new(GameState::AssetLoading)
                    .continue_to(GameState::Title)
                    .track_assets(),
            )
            .add_plugin(state_transition::StateTransitionDetectorPlugin::<GameState>::default());

        app.add_plugin(enemy::EnemyPlugin)
            .add_plugin(player::PlayerPlugin)
            .add_plugin(leaf::LeafPlugin)
            .add_plugin(title::TitlePlugin)
            .add_plugin(gameover::GameOverPlugin)
            // .register_inspectable::<player::Player>()
            // .register_inspectable::<Rotation>()
            .add_startup_system(startup);

        app.add_audio_channel::<BGMTrack>()
            .add_system(bevy::input::system::exit_on_esc_system)
            .init_resource::<MousePos>()
            .init_resource::<GameAssets>()
            .add_system(my_cursor_system)
            .add_system(rotation_system)
            .add_enter_system(GameState::InGame, ingame_startup)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::InGame)
                    .with_system(score_system)
                    .into(),
            );
    }
}

struct GameAssets {
    bgm: Handle<AudioSource>,
    font: Handle<Font>,
}

impl FromWorld for GameAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        let bgm = assets.load("bgm.ogg");
        let font = assets.load("fonts/FiraSans-Bold.ttf");

        let mut loading = world.get_resource_mut::<AssetsLoading>().unwrap();
        loading.add(bgm.clone());
        loading.add(font.clone());

        GameAssets { bgm, font }
    }
}

fn startup(mut commands: Commands, mut windows: ResMut<Windows>) {
    windows.primary_mut().set_resizable(false);
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
    commands.spawn_bundle(UiCameraBundle::default());
}

#[derive(Component)]
struct Score(u32);

fn ingame_startup(
    mut commands: Commands,
    leaf_asset: Res<LeafAsset>,
    audio: Res<AudioChannel<BGMTrack>>,
    assets: Res<GameAssets>,
) {
    let leaf_pos = [
        [0, 0],
        [0, 1],
        [1, 0],
        [0, -1],
        [-1, 0],
        [-1, -1],
        [1, 1],
        [-1, 1],
        [1, -1],
        [0, -2],
        [0, 2],
        [2, 0],
        [-2, 0],
    ];

    let mut leaves = vec![];
    for p in leaf_pos {
        leaves.push(
            leaf::spawn_leaf(&mut commands, IVec2::new(p[0], p[1]), &leaf_asset)
                .insert(InGameTag)
                .id(),
        );
    }

    commands
        .spawn_bundle((
            Name::new("Leafs"),
            GlobalTransform::default(),
            Transform::default(),
        ))
        .insert(InGameTag)
        .push_children(&leaves);

    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..default()
                },
                ..default()
            },
            // Use the `Text::with_section` constructor
            text: Text::with_section(
                "Score: 0",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 40.0,
                    color: Color::SEA_GREEN,
                },
                default(),
            ),
            ..default()
        })
        .insert(Score(0))
        .insert(InGameTag);

    audio.set_volume(0.2);
    audio.play_looped(assets.bgm.clone());
}

fn score_system(mut kill_ev: EventReader<EnemyKillEvent>, mut q: Query<(&mut Text, &mut Score)>) {
    let (mut text, mut score) = q.single_mut();

    score.0 += kill_ev.iter().count() as u32;

    text.sections = vec![TextSection {
        value: format!("Score: {}", score.0),
        style: text.sections[0].style.clone(),
    }]
}

#[derive(Component, Default, Inspectable)]
pub struct Rotation(pub f32);

fn rotation_system(mut q: Query<(&mut Transform, &Rotation), Changed<Rotation>>) {
    q.for_each_mut(|(mut tr, r)| {
        tr.rotation = Quat::from_rotation_z(r.0);
    });
}

#[derive(Component)]
struct MainCamera;

#[derive(Default, Debug)]
struct MousePos(Option<Vec2>);

fn my_cursor_system(
    // need to get window dimensions
    windows: Res<Windows>,
    mut cursor_evr: EventReader<CursorMoved>,
    // query to get camera transform
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut mouse_pos: ResMut<MousePos>,
) {
    if let Some(cursor_moved) = cursor_evr.iter().next_back() {
        let screen_pos = cursor_moved.position;

        // get the camera info and transform
        // assuming there is exactly one main camera entity, so query::single() is OK
        let (camera, camera_transform) = camera.single();

        let wnd = windows.primary();

        // get the size of the window
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix.inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        let new_mouse_pos = world_pos.truncate();

        if mouse_pos.0 != Some(new_mouse_pos) {
            mouse_pos.0 = Some(new_mouse_pos);
        }
    }
}
