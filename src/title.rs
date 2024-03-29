use bevy::{prelude::*, sprite::Anchor};
use bevy_egui::EguiContexts;

use crate::{player::PlayerAssets, GameAssets, MainCamera};

use super::GameState;

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup_title.in_schedule(OnEnter(GameState::Title)))
            .add_system(despawn_title.in_schedule(OnExit(GameState::Title)))
            .add_systems((frog_scale, control).in_set(OnUpdate(GameState::Title)));
    }
}

#[derive(Component)]
struct Title;

#[derive(Component)]
struct Frog;

fn setup_title(
    mut commands: Commands,
    assets: Res<PlayerAssets>,
    mut transform: Query<&mut Transform, With<MainCamera>>,
    game_assets: Res<GameAssets>,
) {
    commands
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..default()
                },
                ..default()
            },
            // Use the `Text::with_section` constructor
            text: Text::from_section(
                // Accepts a `String` or any type that converts into a `String`, such as `&str`
                "LEAFROG",
                TextStyle {
                    font: game_assets.font.clone(),
                    font_size: 150.0,
                    color: Color::SEA_GREEN,
                },
            )
            .with_alignment(TextAlignment::Center),
            ..default()
        })
        .insert(Title);

    let frog = SpriteBundle {
        texture: assets.player[0].clone(),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0))
            .with_scale(Vec3::new(3.0, 3.0, 1.0)),
        sprite: Sprite {
            anchor: Anchor::Custom(Vec2::new(0.0, (64. - 50.) / 64.)),
            ..default()
        },
        ..default()
    };
    commands.spawn((frog, Frog, Title));

    transform.single_mut().translation = Vec3::new(0., 0., 999.0);
}

fn frog_scale(mut frog: Query<&mut Transform, With<Frog>>, time: Res<Time>) {
    let mut tr = frog.single_mut();
    tr.scale = Vec2::splat(3.0 + 0.5 * f32::sin(time.elapsed_seconds() * std::f32::consts::PI))
        .extend(1.0);
}

fn control(
    buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut egui_contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if egui_contexts.ctx_mut().is_pointer_over_area() {
        return;
    }

    if buttons.just_released(MouseButton::Left) || keys.just_released(KeyCode::Space) {
        next_state.set(GameState::InGame);
    }
}

fn despawn_title(mut commands: Commands, q: Query<Entity, With<Title>>) {
    info!("despawn_title");
    q.for_each(|e| commands.entity(e).despawn_recursive());
}
