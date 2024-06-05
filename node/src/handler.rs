use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use std::collections::HashMap;
use std::time::Instant;
use z4_engine::{
    address_hex, hex_address, json, simple_game_result, Address, DefaultParams, Error,
    HandleResult, Handler, PeerId, Result, RoomId, Task, Tasks, Value,
};

const BOARD_SIZE_I: usize = 14;
const BOARD_SIZE_J: usize = 21;
const INIT_POSITIONS: [(usize, usize); 4] = [(0, 0), (13, 0), (0, 20), (13, 20)];
const TIMEOUT: u128 = 300;
const CAKE_TIMEOUT: u64 = 4;
const CAKE_NUMBER: usize = 10;
const INIT_CAKE_TIMEOUT: u64 = 20;
const LOOP_CAKE_TIMEOUT: u64 = 5;

pub struct Player {
    position: (usize, usize),
    score: u32,
    timeout: Instant,
}

pub struct Operation {
    player: PeerId,
    position: (usize, usize),
    eaten: bool,
}

pub struct Cake {
    index: u32,
    position: (usize, usize),
    timeout: Instant,
}

pub struct GameHandler {
    prng: ChaChaRng,
    board: Vec<Vec<f32>>,
    accounts: HashMap<PeerId, (String, Player)>,
    alive_cakes: Vec<Cake>,
    cakes: Vec<Cake>,
    operations: Vec<Operation>,
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

    // TODO over game
    fn over(&self) -> (Vec<u8>, Vec<u8>) {
        let mut players: Vec<(Address, u32)> = self
            .accounts
            .iter()
            .map(|(_, (account, player))| (hex_address(account).unwrap(), player.score))
            .collect();
        players.sort_by(|(_, sa), (_, sb)| sb.cmp(sa));
        let winners: Vec<Address> = players.iter().map(|(a, _s)| *a).collect();

        let rank = simple_game_result(&winners);
        (rank, vec![])
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
                    state.cakes.push(cake);
                } else {
                    break;
                }
            }

            if state.cakes.len() == CAKE_NUMBER {
                // over game
                let (data, proof) = state.over();
                results.over(data, proof);
                over_response(&mut results);
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
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl Handler for GameHandler {
    type Param = DefaultParams;

    async fn create(
        peers: &[(Address, PeerId, [u8; 32])],
        _params: Vec<u8>,
        _rid: RoomId,
        seed: [u8; 32]
    ) -> (Self, Tasks<Self>) {
        let timeout = Instant::now();
        let accounts = peers
            .iter()
            .enumerate()
            .map(|(i, (account, peer, _pk))| {
                (
                    *peer,
                    (
                        address_hex(account),
                        Player {
                            position: INIT_POSITIONS[i],
                            score: 0,
                            timeout,
                        },
                    ),
                )
            })
            .collect();

        // TODO prove rng
        let mut prng = ChaChaRng::from_seed(seed);

        let board = (0..BOARD_SIZE_I)
            .map(|_i| {
                (0..BOARD_SIZE_J)
                    .map(|_j| prng.gen_range(-0.1..0.1))
                    .collect()
            })
            .collect();

        (
            Self {
                prng,
                board,
                accounts,
                alive_cakes: vec![],
                cakes: vec![],
                operations: vec![],
            },
            vec![Box::new(CakeTask(0))],
        )
    }

    async fn online(&mut self, peer: PeerId) -> Result<HandleResult<Self::Param>> {
        println!("Peer: {:?} connected =====", peer);
        let mut result = HandleResult::default();
        let players_status = self.status();

        result.add_one(peer, "connected", DefaultParams(players_status));
        Ok(result)
    }

    async fn handle(
        &mut self,
        player: PeerId,
        method: &str,
        params: DefaultParams,
    ) -> Result<HandleResult<Self::Param>> {
        if let Some((_, p)) = self.accounts.get_mut(&player) {
            if p.timeout.elapsed().as_millis() < TIMEOUT {
                return Err(Error::Timeout);
            } else {
                p.timeout = Instant::now();
            }
        } else {
            return Err(Error::NoPlayer);
        }

        match method {
            "move" => do_move(&mut self, player, params),
            _ => Err(Error::Params),
        }
    }
}

fn do_move(
    handler: &mut GameHandler,
    player: PeerId,
    params: DefaultParams,
) -> Result<HandleResult<DefaultParams>> {
    if params.0.len() != 2 {
        return Err(Error::Params);
    }
    let x = params.0[0].as_u64().unwrap_or(0) as usize;
    let y = params.0[1].as_u64().unwrap_or(0) as usize;
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
            // over game
            let (data, proof) = handler.over();
            results.over(data, proof);
            over_response(&mut results);
        }
    }

    Ok(results)
}

fn move_response(
    results: &mut HandleResult<DefaultParams>,
    account: String,
    position: (usize, usize),
) {
    results.add_all(
        "moved",
        DefaultParams(vec![account.into(), position.0.into(), position.1.into()]),
    );
}

fn cake_response(results: &mut HandleResult<DefaultParams>, index: u32, position: (usize, usize)) {
    results.add_all(
        "cake",
        DefaultParams(vec![index.into(), position.0.into(), position.1.into()]),
    );
}

fn eaten_response(
    results: &mut HandleResult<DefaultParams>,
    index: u32,
    account: String,
    score: u32,
) {
    results.add_all(
        "eaten",
        DefaultParams(vec![index.into(), account.into(), score.into()]),
    );
}

fn over_response(results: &mut HandleResult<DefaultParams>) {
    results.add_all("over", DefaultParams(vec![]));
}
