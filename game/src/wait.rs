use bevy::prelude::*;
use bevy_web3::{EthWallet, RecvError, Token};
use z4_bevy::{FetchRoomStatusTimer, RoomMarket};

use crate::{
    style::{HOVERED_BUTTON, NORMAL_BUTTON, PRESSED_BUTTON},
    Game, GameState,
};

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(FetchRoomStatusTimer::seconds(2.0));
    commands.spawn(CountdownTimer(Timer::from_seconds(
        1.0,
        TimerMode::Repeating,
    )));
}

pub fn cleanup(mut game: ResMut<Game>) {
    game.waiting_entity = None;
}

pub fn show(mut commands: Commands, market: Res<RoomMarket>, mut game: ResMut<Game>) {
    if let Some(entity) = game.waiting_entity {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).despawn_recursive();
        }
    }

    let is_admin = if let Some(waiting) = &market.waiting {
        waiting.players[0] == game.account
    } else {
        false
    };

    game.waiting_entity = Some(
        commands
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                if let Some(waiting) = &market.waiting {
                    parent.spawn(TextBundle::from_section(
                        format!("Waiting room: {}", waiting.room),
                        TextStyle {
                            font_size: 30.,
                            ..default()
                        },
                    ));

                    for player in &waiting.players {
                        parent.spawn(TextBundle::from_section(
                            format!("Player: {}", player),
                            TextStyle {
                                font_size: 30.,
                                ..default()
                            },
                        ));
                    }

                    // show: sequencer
                    parent.spawn(TextBundle::from_section(
                        format!(
                            "Sequencer: {}",
                            waiting.sequencer.as_ref().unwrap_or(&"...".to_owned())
                        ),
                        TextStyle {
                            font_size: 30.,
                            ..default()
                        },
                    ));

                    // show: sequencer
                    parent.spawn(TextBundle::from_section(
                        format!(
                            "Server: {}",
                            waiting.websocket.as_ref().unwrap_or(&"...".to_owned())
                        ),
                        TextStyle {
                            font_size: 30.,
                            ..default()
                        },
                    ));

                    if waiting.websocket.is_some() && game.countdown == 0 {
                        game.countdown = 10;
                        game.room = waiting.room;
                        game.server = waiting.websocket.clone().unwrap();
                    }
                }

                if game.countdown == 0 {
                    if is_admin {
                        parent
                            .spawn(ButtonBundle {
                                style: Style {
                                    width: Val::Px(400.0),
                                    height: Val::Px(65.0),
                                    border: UiRect::all(Val::Px(5.0)),
                                    margin: UiRect::all(Val::Px(20.0)),
                                    // horizontally center child text
                                    justify_content: JustifyContent::Center,
                                    // vertically center child text
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                border_color: BorderColor(Color::BLACK),
                                background_color: NORMAL_BUTTON.into(),
                                ..default()
                            })
                            .with_children(|parent| {
                                parent.spawn(TextBundle::from_section(
                                    "Start Now!",
                                    TextStyle {
                                        font_size: 30.0,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..default()
                                    },
                                ));
                            });
                    }
                } else {
                    // show: sequencer
                    parent.spawn(TextBundle::from_section(
                        format!("Countdown: {}", game.countdown),
                        TextStyle {
                            font_size: 30.,
                            ..default()
                        },
                    ));
                }
            })
            .id(),
    );
}

pub fn _start(wallet: Res<EthWallet>) {
    match wallet.recv_transaction() {
        Ok(tx) => {
            info!("tx: {:?}", tx);
        }
        Err(RecvError::Empty) => {}
        Err(RecvError::Closed) => {}
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct CountdownTimer(Timer);

pub fn countdown(
    time: Res<Time>,
    mut query: Query<&mut CountdownTimer>,
    mut game: ResMut<Game>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for mut timer in &mut query {
        if timer.tick(time.delta()).just_finished() {
            if game.countdown > 0 {
                game.countdown -= 1;
                if game.countdown == 0 {
                    next_state.set(GameState::Playing);
                }
            }
        }
    }
}

pub fn start_button(
    wallet: Res<EthWallet>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    game: Res<Game>,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;

                let data = game
                    .contract
                    .encode("startRoom", &[Token::Uint(game.room.into())]);
                wallet.send(&game.account, game.contract.address, data);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
