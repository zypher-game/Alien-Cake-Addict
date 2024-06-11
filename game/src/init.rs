use bevy::prelude::*;
use bevy_web3::{EthWallet, RecvError};

use crate::{
    style::{HOVERED_BUTTON, NORMAL_BUTTON, PRESSED_BUTTON},
    Game, GameState,
};

pub fn connect_wallet(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Only support metamask & opbnb testnet now",
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
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect {
                            top: Val::Percent(5.),
                            ..default()
                        },
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Connect wallet",
                        TextStyle {
                            font_size: 30.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        });
}

pub fn connect_button(
    wallet: Res<EthWallet>,
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                text.sections[0].value = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
                wallet.connect();
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

pub fn wallet_account(
    mut next_state: ResMut<NextState<GameState>>,
    mut game: ResMut<Game>,
    mut channel: ResMut<EthWallet>,
) {
    match channel.recv_account() {
        Ok((account, network)) => {
            info!("account: {:?}, network: {}", account, network);
            // TODO
            game.account = account;
            game.chain = network;
            next_state.set(GameState::Listing);
        }
        Err(RecvError::Empty) => {}
        Err(RecvError::Closed) => {}
    }
}
