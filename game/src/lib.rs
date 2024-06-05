//! Eat the cakes. Eat them all. An example 3D game.

mod init;
mod list;
mod over;
mod play;
mod style;
mod wait;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_web3::{Contract, EthWallet, WalletPlugin};
use std::collections::{BTreeMap, HashMap};
use z4_bevy::{
    fetch_room_market, fetch_room_status, handle_room_market, handle_room_status, PeerKey, RoomId,
    RoomMarket, Z4ClientPlugin,
};
use z4_types::contracts::SIMPLE_GAME_ABI;

use play::{Cake, Cell, Player};
use style::{BOARD_SIZE_I, BOARD_SIZE_J};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    Initing,
    Listing,
    Waiting,
    Playing,
    GameOver,
}

pub fn start() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(Z4ClientPlugin)
        .add_plugins(WalletPlugin)
        .init_state::<GameState>()
        .insert_resource(Game::init())
        .add_systems(Startup, (setup_2d_cameras, init))
        .add_systems(
            Update,
            (
                init::connect_wallet,
                init::connect_button,
                init::wallet_account,
            )
                .run_if(in_state(GameState::Initing)),
        )
        .add_systems(OnExit(GameState::Initing), teardown)
        .add_systems(OnEnter(GameState::Listing), setup_2d_cameras)
        .add_systems(
            Update,
            (list::show, list::join, list::create, handle_room_market)
                .run_if(in_state(GameState::Listing)),
        )
        .add_systems(
            Update,
            (fetch_room_market, tracing_account)
                .run_if(in_state(GameState::Listing))
                .run_if(on_timer(std::time::Duration::from_secs(2))),
        )
        .add_systems(OnExit(GameState::Listing), (teardown, list::cleanup))
        .add_systems(OnEnter(GameState::Waiting), wait::setup)
        .add_systems(
            Update,
            (
                back_button,
                wait::show,
                wait::start_button,
                wait::countdown,
                fetch_room_status,
                handle_room_status,
            )
                .run_if(in_state(GameState::Waiting)),
        )
        .add_systems(OnExit(GameState::Waiting), (teardown, wait::cleanup))
        .add_systems(OnEnter(GameState::Playing), play::setup)
        .add_systems(
            Update,
            (
                back_button,
                play::move_player,
                play::focus_camera,
                play::rotate_cake,
                play::scoreboard_system,
                play::ws_message,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnExit(GameState::Playing), (teardown, play::cleanup))
        .add_systems(
            OnEnter(GameState::GameOver),
            (setup_2d_cameras, over::display_score),
        )
        .add_systems(
            Update,
            (
                back_button,
                over::gameover_keyboard.run_if(in_state(GameState::GameOver)),
                bevy::window::close_on_esc,
            ),
        )
        .add_systems(OnExit(GameState::GameOver), teardown)
        .run();
}

#[derive(Resource)]
struct Game {
    chain: u64,
    account: String,
    contract: Contract,
    room: RoomId,
    countdown: u32,
    listing_entity: Option<Entity>,
    waiting_entity: Option<Entity>,
    server: String,
    board: Vec<Vec<Cell>>,
    peer: PeerKey,
    player: Player,
    opponents: HashMap<String, Player>,
    cakes: HashMap<u32, Cake>,
    cake_last: Option<Entity>,
    cake_handle: Handle<Scene>,
    scores: BTreeMap<String, u32>,
    camera_should_focus: Vec3,
    camera_is_focus: Vec3,
}

impl Game {
    fn init() -> Game {
        let window = web_sys::window().unwrap();
        let peer = if let Ok(Some(ss)) = window.session_storage() {
            if let Ok(Some(s)) = ss.get("peer-key") {
                let bytes = hex::decode(&s).unwrap();
                PeerKey::from_db_bytes(&bytes).unwrap()
            } else {
                // generate key
                let peer = PeerKey::generate(&mut rand::thread_rng());
                let bytes = peer.to_db_bytes();
                let s = hex::encode(&bytes);
                let _ = ss.set("peer-key", &s);
                peer
            }
        } else {
            PeerKey::generate(&mut rand::thread_rng())
        };

        let address = "0x8d40E727b38F307fc5db83D2AB10B76dc3fA2590"; // opbnb-testnet
        // let address = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"; // localhost

        Game {
            peer,
            contract: Contract::load(address, SIMPLE_GAME_ABI.as_bytes()),
            chain: 0,
            account: Default::default(),
            room: 0,
            countdown: 0,
            listing_entity: None,
            waiting_entity: None,
            server: "".to_owned(),
            board: vec![vec![Cell { height: 0.0 }; BOARD_SIZE_J]; BOARD_SIZE_I],
            player: Player::default(),
            opponents: HashMap::default(),
            cakes: HashMap::default(),
            cake_last: None,
            cake_handle: Default::default(),
            scores: BTreeMap::default(),
            camera_should_focus: Vec3::default(),
            camera_is_focus: Vec3::default(),
        }
    }

    pub fn is_chain(&self) -> bool {
        self.chain == 5611 // opBNB Testnet
    }
}

fn init(mut room_market: ResMut<RoomMarket>, game: Res<Game>) {
    // TODO game init from chain
    room_market.url = "https://aca.zypher.dev/rpc".to_owned(); // testnet
    //room_market.url = "http://127.0.0.1:8080".to_owned(); // localhost

    // setup room_market
    room_market.game = game.contract.address();
    room_market.contract = Contract::load(&room_market.game, SIMPLE_GAME_ABI.as_bytes());
}

fn setup_2d_cameras(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

// remove all entities that are not a camera or window
fn teardown(mut commands: Commands, entities: Query<Entity, Without<Window>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

fn back_button(mut contexts: EguiContexts, mut next_state: ResMut<NextState<GameState>>) {
    egui::Area::new("BACK")
        .fixed_pos(egui::pos2(4.0, 4.0))
        .show(contexts.ctx_mut(), |ui| {
            if ui.button("BACK").clicked() {
                next_state.set(GameState::Listing);
            }
        });
}

fn tracing_account(mut game: ResMut<Game>, mut channel: ResMut<EthWallet>) {
    channel.connect();
    match channel.recv_account() {
        Ok((account, network)) => {
            game.account = account;
            game.chain = network;
        }
        Err(bevy_web3::RecvError::Empty) => {}
        Err(bevy_web3::RecvError::Closed) => {}
    }
}
