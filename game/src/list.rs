use bevy::prelude::*;

use bevy_egui::{egui, EguiContexts};
use bevy_web3::{EthWallet, RecvError, Token, H160, U256};
use z4_bevy::RoomMarket;

use crate::{
    style::{HOVERED_BUTTON, NORMAL_BUTTON, PRESSED_BUTTON},
    Game, GameState,
};

pub fn cleanup(mut game: ResMut<Game>) {
    game.listing_entity = None;
}

pub fn show(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut market: ResMut<RoomMarket>,
    wallet: Res<EthWallet>,
    mut game: ResMut<Game>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(entity) = game.listing_entity {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).despawn_recursive();
        }
    }

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
                parent.spawn(TextBundle::from_section(
                    format!(
                        "Network Chain id: {} - {}",
                        game.chain,
                        if game.is_chain() {
                            "opBNB Testnet"
                        } else {
                            "NOT Support !!!"
                        }
                    ),
                    TextStyle {
                        font_size: 30.,
                        ..default()
                    },
                ));

                parent.spawn(TextBundle::from_section(
                    format!("Account: {}", game.account),
                    TextStyle {
                        font_size: 30.,
                        ..default()
                    },
                ));

                parent.spawn(TextBundle::from_section(
                    format!("Local peer: {}", game.peer.peer_id().to_hex()),
                    TextStyle {
                        font_size: 30.,
                        ..default()
                    },
                ));

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
                            "Create Room",
                            TextStyle {
                                font_size: 30.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ));
                    });
            })
            .id(),
    );

    if game.is_chain() {
        let rooms = market.rooms.clone(); // TODO only clone waiting rooms
        let mut pendings = vec![];
        let mut waitings = vec![];
        let mut playings = vec![];

        for room in rooms {
            if room.websocket.is_some() {
                playings.push(room);
                continue;
            }

            if room.players.contains(&game.account) {
                waitings.push(room);
            } else {
                pendings.push(room)
            }
        }

        egui::Window::new("Pending Rooms").show(contexts.ctx_mut(), |ui| {
            if pendings.is_empty() {
                ui.label("No rooms, waiting...");
            }

            for room in pendings {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Room: {}, players: {}",
                        room.room,
                        room.players.len()
                    ));

                    if ui.button("Join").clicked() {
                        // join room
                        let pid_bytes = game.peer.peer_id().0;
                        let data = game.contract.encode(
                            "joinRoom",
                            &[
                                Token::Uint(room.room.into()),
                                Token::Address(H160(pid_bytes)),
                            ],
                        );
                        wallet.send(&game.account, game.contract.address, data);
                    }
                });
            }
        });

        egui::Window::new("Waiting Rooms").show(contexts.ctx_mut(), |ui| {
            if waitings.is_empty() {
                ui.label("No rooms, waiting...");
            }

            for room in waitings {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Room: {}, players: {}",
                        room.room,
                        room.players.len()
                    ));

                    if ui.button("Waiting").clicked() {
                        game.room = room.room;
                        market.waiting = Some(room.clone());
                        next_state.set(GameState::Waiting);
                    }
                });
            }
        });

        egui::Window::new("Playing Rooms").show(contexts.ctx_mut(), |ui| {
            if playings.is_empty() {
                ui.label("No rooms, waiting...");
            }

            for room in playings {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Room: {}, players: {}",
                        room.room,
                        room.players.len()
                    ));

                    if ui.button("Play").clicked() {
                        game.room = room.room;
                        game.server = room.websocket.clone().unwrap();
                        next_state.set(GameState::Playing);
                    }
                });
            }
        });
    }
}

pub fn join(wallet: Res<EthWallet>) {
    match wallet.recv_transaction() {
        Ok(tx) => {
            info!("tx: {:?}", tx);
            //
        }
        Err(RecvError::Empty) => {}
        Err(RecvError::Closed) => {}
    }
}

pub fn create(
    wallet: Res<EthWallet>,
    game: Res<Game>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;

                // create room
                let pid_bytes = game.peer.peer_id().0;
                let data = game
                    .contract
                    .encode("createRoom", &[
                        Token::Uint(U256::zero()),
                        Token::Bool(false),
                        Token::Address(H160(pid_bytes)),
                        Token::FixedBytes(vec![0u8;32])
                    ]);
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
