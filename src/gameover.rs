use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::AudioChannel;
use iyes_loopless::prelude::*;

use crate::{BGMTrack, GameAssets, GameState, InGameTag};

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameOverState>()
            .add_enter_system(GameState::GameOver, setup_gameover)
            .add_exit_system(GameState::GameOver, despawn::<GameOverTag>)
            .add_system(control.run_in_state(GameState::GameOver))
            .add_exit_system(GameState::GameOver, despawn::<InGameTag>);
    }
}

#[derive(Component)]
struct GameOverTag;

#[derive(Default)]
struct GameOverState {
    cooldown: Timer,
}

fn setup_gameover(
    mut commands: Commands,
    audio: Res<AudioChannel<BGMTrack>>,
    mut state: ResMut<GameOverState>,
    game_assets: Res<GameAssets>,
) {
    state.cooldown = Timer::new(Duration::from_millis(800), false);

    commands
        .spawn_bundle(TextBundle {
            style: Style {
                size: Size::new(Val::Percent(50.0), Val::Px(0.)),
                margin: Rect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            text: Text::with_section(
                "GAMEOVER",
                TextStyle {
                    font: game_assets.font.clone(),
                    font_size: 150.0,
                    color: Color::SEA_GREEN,
                },
                TextAlignment {
                    vertical: VerticalAlign::Center,
                    horizontal: HorizontalAlign::Center,
                },
            ),
            ..default()
        })
        .insert(GameOverTag);

    audio.stop();
}

fn control(
    mut commands: Commands,
    buttons: Res<Input<MouseButton>>,
    mut state: ResMut<GameOverState>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
) {
    state.cooldown.tick(time.delta());
    if !state.cooldown.finished() {
        return;
    }

    if buttons.just_released(MouseButton::Left) || keys.just_released(KeyCode::Space) {
        commands.insert_resource(NextState(GameState::Title));
    }
}

fn despawn<C: Component>(mut commands: Commands, q: Query<Entity, With<C>>) {
    q.for_each(|e| commands.entity(e).despawn_recursive());
}
