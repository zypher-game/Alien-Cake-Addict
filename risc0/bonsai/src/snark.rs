use ::bonsai_sdk::alpha::{responses::SnarkReceipt, SessionId};
use bonsai_sdk::alpha as bonsai_sdk;
use poker_core::errors::{PokerError, Result};
use std::time::Duration;

pub fn stark_to_snark(session_id: SessionId) -> Result<SnarkReceipt> {
    let client = bonsai_sdk::Client::from_env(risc0_zkvm::VERSION)
        .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

    let snark_session = client
        .create_snark(session_id.uuid)
        .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

    loop {
        let res = snark_session
            .status(&client)
            .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

        match res.status.as_str() {
            "RUNNING" => {
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
            "SUCCEEDED" => {
                return Ok(res.output.unwrap());
            }
            _ => {
                return Err(PokerError::BonsaiSdkError(format!(
                    "Workflow exited: {} - | err: {}",
                    res.status,
                    res.error_msg.unwrap_or_default()
                )));
            }
        }
    }
}

#[cfg(test)]
#[allow(unused)]
mod test {
    use crate::{snark::stark_to_snark, stark::prove_bonsai};
    use ark_bn254::Fr;
    use ark_ff::{BigInteger, PrimeField};
    use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
    use hex::FromHex;
    use num_bigint::BigInt;
    use poker_core::{
        mock_data::{journal::mock_journal, task::mock_task, task0::mock_task0},
        task::{Task0, TaskCommit},
    };
    use poker_methods::POKER_METHOD_ID;
    use risc0_zkvm::{
        serde::{from_slice, to_vec},
        sha::{Digest, Digestible},
        CompactReceipt, Groth16Seal, InnerReceipt, Journal, Receipt, ALLOWED_IDS_ROOT,
    };
    use std::{str::FromStr, time::Instant};

    #[test]
    #[cfg(all(feature = "serialize0", feature = "deserialize0"))]
    fn stark_to_snark_test() {
        dotenv::dotenv().ok();
        let task_bytes = mock_task0();
        let task: Task0 = from_slice(&task_bytes).unwrap();

        let start = Instant::now();
        let (receipt, session_id) = prove_bonsai(&task).unwrap();
        println!("Prover time: {:.2?}", start.elapsed());

        assert!(receipt.verify(POKER_METHOD_ID).is_ok());

        let commit: TaskCommit = receipt.journal.decode().unwrap();
        assert_eq!(commit.room_id, task.room_id);
        assert_eq!(commit.players_hand, task.players_hand);
        assert_eq!(commit.first_player, task.first_player);

        let start = Instant::now();
        let snark_proof = stark_to_snark(session_id).unwrap();
        println!("Stark2Snark time: {:.2?}", start.elapsed());

        {
            let receipt_claim = receipt.get_claim().unwrap();
            let receipt = Receipt::new(
                InnerReceipt::Compact(CompactReceipt {
                    seal: snark_proof.snark.to_vec(),
                    claim: receipt_claim.clone(),
                }),
                receipt.journal.bytes,
            );

            assert!(receipt.verify(POKER_METHOD_ID).is_ok());
        }
    }

    #[test]
    #[cfg(all(feature = "serialize0", feature = "deserialize0"))]
    fn onchain_verify_test() {
        dotenv::dotenv().ok();
        let task_bytes = mock_task0();
        let task: Task0 = from_slice(&task_bytes).unwrap();

        let start = Instant::now();
        let (receipt, session_id) = prove_bonsai(&task).unwrap();
        println!("Prover time: {:.2?}", start.elapsed());

        assert!(receipt.verify(POKER_METHOD_ID).is_ok());

        let journal: Vec<u8> = bytemuck::cast_slice(&mock_journal()).to_vec();
        assert_eq!(journal, receipt.journal.bytes);

        let commit: TaskCommit = receipt.journal.decode().unwrap();
        assert_eq!(commit.room_id, task.room_id);
        assert_eq!(commit.players_hand, task.players_hand);
        assert_eq!(commit.first_player, task.first_player);

        let start = Instant::now();
        let snark_proof = stark_to_snark(session_id).unwrap();
        println!("Stark2Snark time: {:.2?}", start.elapsed());

        let receipt_claim = receipt.get_claim().unwrap();

        let image_id: Digest = POKER_METHOD_ID.into();

        println!("---------on-chain verification data---------");
        println!("seal:0x{}", hex::encode(snark_proof.snark.to_vec()));
        println!("image_id:0x{}", image_id);
        println!("post_digest:0x{}", receipt_claim.post.digest());
        println!("jounral:0x{}", receipt.journal.digest());
    }

    #[test]
    #[cfg(all(feature = "serialize0", feature = "deserialize0"))]
    fn journal_test() {
        let journal_byte32 = mock_journal();
        // println!("{:?}",journal_byte32);
        let journal_bytes: Vec<u8> = bytemuck::cast_slice(&journal_byte32).to_vec();
        println!("{:?}", journal_bytes);
        let commit: TaskCommit = from_slice(&journal_byte32).unwrap();

        let vec = to_vec(&commit).unwrap();

        let x0 = commit.players_hand[0][0].0.e1;
        let mut y0 = vec![];
        x0.serialize_uncompressed(&mut y0).unwrap();

        let x1 = commit.players_hand[0][0].0.e2;
        let mut y1 = vec![];
        x1.serialize_uncompressed(&mut y1).unwrap();

        println!("y0:{:?}", y0);
        println!("y1:{:?}", y1);

        //  println!("{:?}",commit);
    }
}
