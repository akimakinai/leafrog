use std::time::Duration;

use bevy::{prelude::*, audio::AudioSink};
use bevy_egui::EguiContexts;

use crate::{BGMTrack, GameAssets, GameState, InGameTag};

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameOverState>()
            .add_system(setup_gameover.in_schedule(OnEnter(GameState::GameOver)))
            .add_systems((despawn::<InGameTag>, despawn::<GameOverTag>).in_schedule(OnExit(GameState::GameOver)))
            .add_system(control.in_set(OnUpdate(GameState::GameOver)));
    }
}

#[derive(Component)]
struct GameOverTag;

#[derive(Default, Resource)]
struct GameOverState {
    cooldown: Timer,
}

fn setup_gameover(
    mut commands: Commands,
    audio_sinks: Res<Assets<AudioSink>>,
    mut bgm: ResMut<BGMTrack>,
    mut state: ResMut<GameOverState>,
    game_assets: Res<GameAssets>,
) {
    info!("setup_gameover");

    state.cooldown = Timer::new(Duration::from_millis(800), default());

    commands
        .spawn(TextBundle {
            style: Style {
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            text: Text::from_section(
                "GAMEOVER",
                TextStyle {
                    font: game_assets.font.clone(),
                    font_size: 150.0,
                    color: Color::SEA_GREEN,
                },
            )
            .with_alignment(TextAlignment::Center),
            ..default()
        })
        .insert(GameOverTag);

    bgm.stop(&audio_sinks);
}

fn control(
    buttons: Res<Input<MouseButton>>,
    mut state: ResMut<GameOverState>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut egui_contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
) {
    state.cooldown.tick(time.delta());
    if !state.cooldown.finished() {
        return;
    }

    if egui_contexts.ctx_mut().is_pointer_over_area() {
        return;
    }

    if buttons.just_released(MouseButton::Left) || keys.just_released(KeyCode::Space) {
        next_state.set(GameState::Title);
    }
}

fn despawn<C: Component>(mut commands: Commands, q: Query<Entity, With<C>>) {
    q.for_each(|e| commands.entity(e).despawn());
}
