use risc0_zkvm::guest::env;
use ethers_core::{types::Address, abi::{encode, Token}};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub enum Operation {
    // player, position
    Move(Address, usize, usize),
    // index, position
    CakeCreated(u32, usize, usize),
    // index
    CakeMissed(u32),
}

fn simple_game_result(ranks: &[Address]) -> Vec<u8> {
    encode(&[Token::Array(
        ranks.iter().map(|v| Token::Address(*v)).collect(),
    )])
}

fn main() {
    println!("read cycle:{}", env::cycle_count());
    // read the input
    let operations: Vec<Operation> = env::read();

    println!("read cycle:{}", env::cycle_count());

    let mut cakes: HashMap<u32, (usize, usize)> = HashMap::new();
    let mut scores: HashMap<Address, u32> = HashMap::new();
    for op in operations {
        match op {
            Operation::Move(player, x, y) => {
                let mut clears: Vec<u32> = vec![];
                for (i, (x1, y1)) in cakes.iter() {
                    if *x1 == x && *y1 == y {
                        clears.push(*i)
                    }
                }

                loop {
                    if let Some(next) = clears.pop() {
                        cakes.remove(&next);
                        scores.entry(player).and_modify(|s| {
                            *s += 1
                        }).or_insert(1);
                    } else {
                        break;
                    }
                }
            }
            Operation::CakeCreated(index, x, y) => {
                cakes.insert(index, (x, y));
            }
            Operation::CakeMissed(index) => {
                let _ = cakes.remove(&index);
            }
        }
    }

    // write public output to the journal
    let mut players: Vec<(Address, u32)> = scores
        .iter()
        .filter_map(|(account, score)| if *score > 0 { Some((*account, *score)) } else { None })
        .collect();
    players.sort_by(|(_, sa), (_, sb)| sb.cmp(sa));
    let winners: Vec<Address> = players.iter().map(|(a, _s)| *a).collect();
    let rank = simple_game_result(&winners);
    env::commit(&rank);

    println!("read cycle:{}", env::cycle_count());
}
