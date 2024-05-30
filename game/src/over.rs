use bevy::prelude::*;

use crate::{Game, GameState};

// restart the game when pressing spacebar
pub fn gameover_keyboard(
    mut next_state: ResMut<NextState<GameState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Listing);
    }
}

// display the number of cake eaten before losing
pub fn display_score(mut commands: Commands, game: Res<Game>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                format!("Cake eaten: {}", game.scores[&game.account]),
                TextStyle {
                    font_size: 80.0,
                    color: Color::rgb(0.5, 0.5, 1.0),
                    ..default()
                },
            ));
        });
}
