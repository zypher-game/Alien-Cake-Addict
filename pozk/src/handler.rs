use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use std::collections::HashMap;
use std::time::Instant;
use serde_json::{Value, json};
use z4_pozk::types::{
    simple_game_result, MethodValues, PeerId, RoomId, Task, Tasks,
    Address, Error,HandleResult, Handler, Result, Player
};
use serde::Serialize;
use chrono::prelude::Utc;

const BOARD_SIZE_I: usize = 14;
const BOARD_SIZE_J: usize = 21;
const INIT_POSITIONS: [(usize, usize); 4] = [(0, 0), (13, 0), (0, 20), (13, 20)];
const TIMEOUT: u128 = 300;
const CAKE_TIMEOUT: u64 = 4;
const CAKE_NUMBER: usize = 10;
const INIT_CAKE_TIMEOUT: u64 = 10;
const LOOP_CAKE_TIMEOUT: u64 = 5;

const MAX_WAITING_TIME: i64 = 120; // 2min

pub struct GamePlayer {
    position: (usize, usize),
    score: u32,
    timeout: Instant,
}

#[derive(Serialize)]
pub enum Operation {
    // player, position
    Move(Address, usize, usize),
    // index, position
    CakeCreated(u32, usize, usize),
    // index
    CakeMissed(u32),
}

pub struct Cake {
    index: u32,
    position: (usize, usize),
    timeout: Instant,
}

pub struct GameHandler {
    prng: ChaChaRng,
    board: Vec<Vec<f32>>,
    accounts: HashMap<PeerId, (String, GamePlayer)>,
    alive_cakes: Vec<Cake>,
    cakes: Vec<Cake>,
    operations: Vec<Operation>,
    overtime: i64,
    started: bool,
    over: bool,
}

impl GameHandler {
    fn status(&self) -> Vec<Value> {
        let mut players: Vec<Value> = self
            .accounts
            .iter()
            .map(|(_pid, (aid, p))| json!((aid.clone(), p.position.0, p.position.1, p.score)))
            .collect();

        let board_s: Vec<Vec<String>> = self
            .board
            .iter()
            .map(|i| i.iter().map(|j| format!("{}", j)).collect())
            .collect();
        let mut boards_string = vec![];
        for i in board_s {
            boards_string.push(i.join(","));
        }
        players.push(json!(boards_string.join(",")));
        players
    }
}

struct CakeTask(usize);

#[async_trait::async_trait]
impl Task for CakeTask {
    type H = GameHandler;

    // TODO 1s to run this task
    fn timer(&self) -> u64 {
        if self.0 == 0 {
            INIT_CAKE_TIMEOUT
        } else {
            LOOP_CAKE_TIMEOUT
        }
    }

    async fn run(
        &mut self,
        state: &mut Self::H,
    ) -> Result<HandleResult<<Self::H as Handler>::Param>> {
        let mut results = HandleResult::default();
        if !state.started {
            let now = Utc::now().timestamp();
            if now < state.overtime {
                // start it when over waiting time
                state.started = true;
                results.started();
            }
            return Ok(results);
        }

        if state.over {
            return Err(Error::Timeout);
        }

        // clear no-alive cakes
        let mut clears: Vec<usize> = vec![];
        for (i, cake) in state.alive_cakes.iter().enumerate() {
            if cake.timeout.elapsed().as_secs() >= CAKE_TIMEOUT {
                clears.push(i);
            } else {
                break;
            }
        }
        if !clears.is_empty() {
            loop {
                if let Some(next) = clears.pop() {
                    let cake = state.alive_cakes.remove(next);
                    eaten_response(&mut results, cake.index, Default::default(), 0);
                    state.operations.push(Operation::CakeMissed(cake.index));
                    state.cakes.push(cake);
                } else {
                    break;
                }
            }

            if state.cakes.len() == CAKE_NUMBER {
                // over game
                state.over = true;
                over_response(&mut results);
                results.over();
            }
        }

        if self.0 < CAKE_NUMBER {
            self.0 += 1;

            // random cake postion
            let i: usize = state.prng.gen_range(0..14);
            let j: usize = state.prng.gen_range(0..21);

            // create new cake
            let index = self.0 as u32;
            let position = (i, j);
            let timeout = Instant::now();
            state.alive_cakes.push(Cake {
                index,
                position,
                timeout,
            });

            // broadcast
            cake_response(&mut results, index, position);
            state.operations.push(Operation::CakeCreated(index, position.0, position.1));
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl Handler for GameHandler {
    type Param = MethodValues;

    fn viewable() -> bool {
        true
    }

    async fn pozk_create(
        player: Player,
        _params: Vec<u8>,
        _room: RoomId,
    ) -> Option<(Self, Tasks<Self>)> {
        let new_player = GamePlayer {
            position: INIT_POSITIONS[0],
            score: 0,
            timeout: Instant::now(),
        };

        let mut accounts = HashMap::new();
        let account = format!("{:?}", player.account);
        accounts.insert(player.peer, (account, new_player));

        let now = Utc::now().timestamp();
        let mut prng = ChaChaRng::from_entropy();
        let board = (0..BOARD_SIZE_I)
            .map(|_i| {
                (0..BOARD_SIZE_J)
                    .map(|_j| prng.gen_range(-0.1..0.1))
                    .collect()
            })
            .collect();

        Some((
            Self {
                prng,
                board,
                accounts,
                alive_cakes: vec![],
                cakes: vec![],
                operations: vec![],
                started: false,
                over: false,
                overtime: now + MAX_WAITING_TIME,
            },
            vec![Box::new(CakeTask(0))],
        ))
    }

    async fn pozk_join(
        &mut self,
        player: Player,
        _params: Vec<u8>,
    ) -> Result<HandleResult<Self::Param>> {
        if self.started {
            return Ok(HandleResult::default());
        }

        let i = self.accounts.len();
        let new_player = GamePlayer {
            position: INIT_POSITIONS[i],
            score: 0,
            timeout: Instant::now(),
        };
        let account = format!("{:?}", player.account);
        self.accounts.insert(player.peer, (account, new_player));

        let mut results = HandleResult::default();
        if self.accounts.len() == 4 {
            self.started = true;
            results.started();
        }

        Ok(results)
    }

    async fn online(&mut self, peer: PeerId) -> Result<HandleResult<Self::Param>> {
        println!("Peer: {:?} connected =====", peer);
        let mut result = HandleResult::default();
        let players_status = self.status();

        result.add_one(peer, MethodValues::new("connected", players_status));
        Ok(result)
    }

    async fn handle(
        &mut self,
        peer: PeerId,
        param: Self::Param,
    ) -> Result<HandleResult<Self::Param>> {
        if let Some((_, p)) = self.accounts.get_mut(&peer) {
            if p.timeout.elapsed().as_millis() < TIMEOUT {
                return Err(Error::Timeout);
            } else {
                p.timeout = Instant::now();
            }
        } else {
            return Err(Error::NoPlayer);
        }

        let MethodValues { method, params } = param;
        match method.as_str() {
            "move" => do_move(&mut self, peer, params),
            _ => Err(Error::Params),
        }
    }

    async fn prove(&mut self) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut players: Vec<(Address, u32)> = self
            .accounts
            .iter()
            .map(|(_, (account, player))| (account.parse().unwrap(), player.score))
            .collect();
        players.sort_by(|(_, sa), (_, sb)| sb.cmp(sa));
        let winners: Vec<Address> = players.iter().map(|(a, _s)| *a).collect();

        let rank = simple_game_result(&winners);
        let proof = vec![];

        Ok((rank, proof))
    }
}

fn do_move(
    handler: &mut GameHandler,
    player: PeerId,
    params: Vec<Value>,
) -> Result<HandleResult<MethodValues>> {
    if params.len() != 2 {
        return Err(Error::Params);
    }
    let x = params[0].as_u64().unwrap_or(0) as usize;
    let y = params[1].as_u64().unwrap_or(0) as usize;
    let position = (x, y);

    // TODO Check new position is valid
    let (aid, p) = handler.accounts.get_mut(&player).unwrap(); // safe
    p.position.0 = position.0;
    p.position.1 = position.1;
    let account = aid.clone();

    //  check if eaten
    let mut clears: Vec<usize> = vec![];
    for (i, cake) in handler.alive_cakes.iter().enumerate() {
        if cake.position == position {
            clears.push(i)
        }
    }

    let mut results = HandleResult::default();
    move_response(&mut results, account.clone(), position);
    handler.operations.push(Operation::Move(account.parse().unwrap(), position.0, position.1));

    if !clears.is_empty() {
        loop {
            if let Some(next) = clears.pop() {
                let cake = handler.alive_cakes.remove(next);
                p.score += 1;
                eaten_response(&mut results, cake.index, account.clone(), p.score);
                handler.cakes.push(cake);
            } else {
                break;
            }
        }

        if handler.cakes.len() == CAKE_NUMBER {
            handler.over = true;
            over_response(&mut results);
            results.over();
        }
    }

    Ok(results)
}

fn move_response(
    results: &mut HandleResult<MethodValues>,
    account: String,
    position: (usize, usize),
) {
    results.add_all(
        MethodValues::new(
            "moved",
            vec![account.into(), position.0.into(), position.1.into()]
        )
    );
}

fn cake_response(results: &mut HandleResult<MethodValues>, index: u32, position: (usize, usize)) {
    results.add_all(
        MethodValues::new(
            "cake",
            vec![index.into(), position.0.into(), position.1.into()]
        )
    );
}

fn eaten_response(
    results: &mut HandleResult<MethodValues>,
    index: u32,
    account: String,
    score: u32,
) {
    results.add_all(
        MethodValues::new(
            "eaten",
            vec![index.into(), account.into(), score.into()]
        ),
    );
}

fn over_response(results: &mut HandleResult<MethodValues>) {
    results.add_all(
        MethodValues::new(
            "over",
            vec![]
        )
    );
}
