use bevy::prelude::*;
use serde::Deserialize;
use std::f32::consts::PI;
use z4_bevy::{build_request, parse_response, RecvError};

#[cfg(target_arch = "wasm32")]
use z4_bevy::wasm::{ws_connect, WsConnection};

#[cfg(not(target_arch = "wasm32"))]
use z4_bevy::ws::{ws_connect, WsConnection};

use crate::{
    style::{BOARD_SIZE_I, BOARD_SIZE_J, RESET_FOCUS},
    Game, GameState,
};

#[derive(Clone)]
pub struct Cell {
    pub height: f32,
}

#[derive(Default, Debug)]
pub struct Player {
    entity: Option<Entity>,
    i: usize,
    j: usize,
}

pub struct Cake {
    entity: Entity,
}

pub fn setup(mut commands: Commands, mut game: ResMut<Game>) {
    game.camera_should_focus = Vec3::from(RESET_FOCUS);
    game.camera_is_focus = game.camera_should_focus;
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(
            -(BOARD_SIZE_I as f32 / 2.0),
            2.0 * BOARD_SIZE_J as f32 / 3.0,
            BOARD_SIZE_J as f32 / 2.0 - 0.5,
        )
        .looking_at(game.camera_is_focus, Vec3::Y),
        ..default()
    });

    ws_connect(&mut commands, &game.server, &game.peer, game.room);
}

pub fn cleanup(mut game: ResMut<Game>) {
    game.cakes.clear();
    game.cake_last = None;
}

pub fn play_setup(mut commands: Commands, mut game: ResMut<Game>, asset_server: Res<AssetServer>) {
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 10.0, 4.0),
        point_light: PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            range: 30.0,
            ..default()
        },
        ..default()
    });

    // spawn the game board
    let cell_scene = asset_server.load("models/AlienCake/tile.glb#Scene0");
    for i in 0..BOARD_SIZE_I {
        for j in 0..BOARD_SIZE_J {
            let height = game.board[i][j].height;
            commands.spawn(SceneBundle {
                transform: Transform::from_xyz(i as f32, height - 0.2, j as f32),
                scene: cell_scene.clone(),
                ..default()
            });
        }
    }

    // spawn the game character
    game.player.entity = Some(
        commands
            .spawn(SceneBundle {
                transform: Transform {
                    translation: Vec3::new(
                        game.player.i as f32,
                        game.board[game.player.i][game.player.j].height,
                        game.player.j as f32,
                    ),
                    rotation: Quat::from_rotation_y(-PI / 2.),
                    ..default()
                },
                scene: asset_server.load("models/AlienCake/alien.glb#Scene0"),
                ..default()
            })
            .id(),
    );

    let mut entities = vec![];
    for (i, opponent) in game.opponents.iter() {
        entities.push((
            i.to_owned(),
            Some(
                commands
                    .spawn(SceneBundle {
                        transform: Transform {
                            translation: Vec3::new(
                                opponent.i as f32,
                                game.board[opponent.i][opponent.j].height,
                                opponent.j as f32,
                            ),
                            rotation: Quat::from_rotation_y(-PI / 2.),
                            ..default()
                        },
                        scene: asset_server.load("models/AlienCake/alien.glb#Scene0"),
                        ..default()
                    })
                    .id(),
            ),
        ));
    }

    for (i, entity) in entities {
        game.opponents.get_mut(&i).map(|v| v.entity = entity);
    }

    // load the scene for the cake
    game.cake_handle = asset_server.load("models/AlienCake/cakeBirthday.glb#Scene0");

    // scoreboard
    commands.spawn(
        TextBundle::from_section(
            "Score:",
            TextStyle {
                font_size: 30.0,
                color: Color::rgb(0.5, 0.5, 1.0),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(50.0),
            ..default()
        }),
    );
}

// control the game character
pub fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game: ResMut<Game>,
    connections: Query<(Entity, &WsConnection)>,
) {
    let mut new_i = game.player.i;
    let mut new_j = game.player.j;

    let mut moved = false;
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        if game.player.i < BOARD_SIZE_I - 1 {
            new_i += 1;
            moved = true;
        }
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        if game.player.i > 0 {
            new_i -= 1;
            moved = true;
        }
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        if game.player.j < BOARD_SIZE_J - 1 {
            new_j += 1;
            moved = true;
        }
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        if game.player.j > 0 {
            new_j -= 1;
            moved = true;
        }
    }

    if moved {
        if let Ok((_, conn)) = connections.get_single() {
            conn.send(build_request(
                "move",
                vec![new_i.into(), new_j.into()],
                &game.peer,
                game.room,
            ));
        }
    }
}

// change the focus of the camera
pub fn focus_camera(
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut transforms: ParamSet<(Query<&mut Transform, With<Camera3d>>, Query<&Transform>)>,
) {
    const SPEED: f32 = 2.0;
    // if there is both a player and a cake, target the mid-point of them
    if let (Some(player_entity), Some(cake_entity)) = (game.player.entity, game.cake_last) {
        let transform_query = transforms.p1();
        if let (Ok(player_transform), Ok(cake_transform)) = (
            transform_query.get(player_entity),
            transform_query.get(cake_entity),
        ) {
            game.camera_should_focus = player_transform
                .translation
                .lerp(cake_transform.translation, 0.5);
        }
        // otherwise, if there is only a player, target the player
    } else if let Some(player_entity) = game.player.entity {
        if let Ok(player_transform) = transforms.p1().get(player_entity) {
            game.camera_should_focus = player_transform.translation;
        }
        // otherwise, target the middle
    } else {
        game.camera_should_focus = Vec3::from(RESET_FOCUS);
    }
    // calculate the camera motion based on the difference between where the camera is looking
    // and where it should be looking; the greater the distance, the faster the motion;
    // smooth out the camera movement using the frame time
    let mut camera_motion = game.camera_should_focus - game.camera_is_focus;
    if camera_motion.length() > 0.2 {
        camera_motion *= SPEED * time.delta_seconds();
        // set the new camera's actual focus
        game.camera_is_focus += camera_motion;
    }
    // look at that new camera's actual focus
    for mut transform in transforms.p0().iter_mut() {
        *transform = transform.looking_at(game.camera_is_focus, Vec3::Y);
    }
}

// let the cake turn on itself
pub fn rotate_cake(game: Res<Game>, time: Res<Time>, mut transforms: Query<&mut Transform>) {
    for (index, cake) in game.cakes.iter() {
        if let Ok(mut cake_transform) = transforms.get_mut(cake.entity) {
            cake_transform.rotate_y(time.delta_seconds());
            cake_transform.scale =
                Vec3::splat(1.0 + (*index as f32 / 10.0 * time.elapsed_seconds().sin()).abs());
        }
    }
}

// update the score displayed during the game
pub fn scoreboard_system(game: Res<Game>, mut query: Query<&mut Text>) {
    for mut text in &mut query {
        let mut string = String::from("Score: ");
        for (p, s) in &game.scores {
            string += &format!("{}...{}:{} ", &p[0..4], &p[40..], s);
        }
        text.sections[0].value = string;
    }
}

/// player id, init position, init score
#[derive(Deserialize, Default)]
struct PlayerStatus(String, usize, usize, u32);

/// player id, position
#[derive(Deserialize, Default)]
struct PlayerMove(String, usize, usize);

/// cake index, cake position
#[derive(Deserialize, Default)]
struct CakeAppear(u32, usize, usize);

/// cake index, peer id, peer score
#[derive(Deserialize, Default)]
struct CakeEaten(u32, String, u32);

pub fn ws_message(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut next_state: ResMut<NextState<GameState>>,
    asset_server: Res<AssetServer>,
    mut transforms: Query<&mut Transform>,
    connections: Query<(Entity, &WsConnection)>,
) {
    if let Ok((entity, conn)) = connections.get_single() {
        match conn.recv() {
            Ok(message) => match parse_response(&message) {
                Ok((_room, method, mut params)) => match method.as_str() {
                    "connected" => {
                        // setup game board
                        let boardv = params.pop().unwrap_or(Default::default());
                        let boards: Vec<Cell> = boardv
                            .as_str()
                            .unwrap_or("")
                            .split(",")
                            .map(|v| Cell {
                                height: v.parse::<f32>().unwrap_or(0.0),
                            })
                            .collect();

                        game.board.clear();
                        for j in boards.chunks(BOARD_SIZE_J) {
                            game.board.push(j.to_vec());
                        }

                        // setup game players & scores
                        game.opponents.clear();
                        game.scores.clear();
                        for param in params {
                            let ps =
                                serde_json::from_value(param).unwrap_or(PlayerStatus::default());
                            if ps.0 == game.account {
                                game.player.i = ps.1;
                                game.player.j = ps.2;
                            } else {
                                let mut player = Player::default();
                                player.i = ps.1;
                                player.j = ps.2;
                                game.opponents.insert(ps.0.clone(), player);
                            }

                            game.scores.insert(ps.0, ps.3);
                        }
                        play_setup(commands, game, asset_server);
                    }
                    "moved" => {
                        if params.len() != 3 || game.player.entity.is_none() {
                            return;
                        }
                        let player = params[0].as_str().unwrap_or("");
                        let new_i = params[1].as_u64().unwrap_or(0) as usize;
                        let new_j = params[2].as_u64().unwrap_or(0) as usize;

                        // Set new position
                        let (old_i, old_j, entity) = if player == &game.account {
                            let old = (game.player.i, game.player.j);
                            game.player.i = new_i;
                            game.player.j = new_j;
                            (old.0, old.1, game.player.entity.unwrap())
                        } else {
                            if let Some(p) = game.opponents.get_mut(player) {
                                let old = (p.i, p.j);
                                p.i = new_i;
                                p.j = new_j;
                                (old.0, old.1, p.entity.unwrap())
                            } else {
                                return;
                            }
                        };

                        // move rotation
                        let mut rotation = 0.0;
                        if old_i > new_i {
                            rotation = -PI / 2.;
                        }
                        if old_i < new_i {
                            rotation = PI / 2.;
                        }
                        if old_j < new_j {
                            rotation = PI;
                        }
                        if old_j > new_j {
                            rotation = 0.0;
                        }

                        // move transforms
                        *transforms.get_mut(entity).unwrap() = Transform {
                            translation: Vec3::new(
                                new_i as f32,
                                game.board[new_i][new_j].height,
                                new_j as f32,
                            ),
                            rotation: Quat::from_rotation_y(rotation),
                            ..default()
                        };
                    }
                    "cake" => {
                        if params.len() != 3 {
                            return;
                        }
                        let index = params[0].as_u64().unwrap_or(0) as u32;
                        let i = params[1].as_u64().unwrap_or(0) as usize;
                        let j = params[2].as_u64().unwrap_or(0) as usize;

                        let entity = commands
                            .spawn(SceneBundle {
                                transform: Transform::from_xyz(
                                    i as f32,
                                    game.board[i][j].height + 0.2,
                                    j as f32,
                                ),
                                scene: game.cake_handle.clone(),
                                ..default()
                            })
                            .with_children(|children| {
                                children.spawn(PointLightBundle {
                                    point_light: PointLight {
                                        color: Color::rgb(1.0, 1.0, 0.0),
                                        intensity: 500_000.0,
                                        range: 10.0,
                                        ..default()
                                    },
                                    transform: Transform::from_xyz(0.0, 2.0, 0.0),
                                    ..default()
                                });
                            })
                            .id();

                        game.cake_last = Some(entity);
                        game.cakes.insert(index, Cake { entity });
                    }
                    "eaten" => {
                        if params.len() != 3 {
                            return;
                        }
                        let index = params[0].as_u64().unwrap_or(0) as u32;
                        let player = params[1].as_str().unwrap_or("");
                        let score = params[2].as_u64().unwrap_or(0) as u32;

                        // clear cake
                        if let Some(cake) = game.cakes.remove(&index) {
                            commands.entity(cake.entity).despawn_recursive();
                            if let Some(last) = game.cake_last {
                                if last == cake.entity {
                                    game.cake_last = None;
                                }
                            }
                        }

                        // update scores
                        if !player.is_empty() {
                            game.scores.get_mut(player).map(|val| {
                                *val = score;
                            });
                        }
                    }
                    "over" => {
                        next_state.set(GameState::GameOver);
                    }
                    _ => {}
                },
                Err(err) => error!("WS: {}, message: {}", err, message),
            },
            Err(RecvError::Empty) => {}
            Err(RecvError::Closed) => commands.entity(entity).despawn(),
        }
    }
}
