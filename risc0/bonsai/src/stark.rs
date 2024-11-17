use ::bonsai_sdk::alpha::SessionId;
use bonsai_sdk::alpha as bonsai_sdk;
use poker_core::{
    errors::{PokerError, Result},
    task::Task0,
};
use poker_methods::POKER_METHOD_ELF;
use risc0_zkvm::{compute_image_id, serde::to_vec, Receipt};
use std::time::Duration;

pub fn prove_bonsai(input_data: &Task0) -> Result<(Receipt, SessionId)> {
    let client = bonsai_sdk::Client::from_env(risc0_zkvm::VERSION)
        .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

    let image_id = hex::encode(compute_image_id(POKER_METHOD_ELF).unwrap());
    client
        .upload_img(&image_id, POKER_METHOD_ELF.to_vec())
        .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

    let input_data = to_vec(&input_data).unwrap();
    let input_data = bytemuck::cast_slice(&input_data).to_vec();
    let input_id = client.upload_input(input_data).unwrap();

    let assumptions: Vec<String> = vec![];

    let session = client
        .create_session(image_id, input_id, assumptions)
        .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

    loop {
        let res = session
            .status(&client)
            .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;

        if res.status == "RUNNING" {
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }

        if res.status == "SUCCEEDED" {
            let receipt_url = res
                .receipt_url
                .expect("API error, missing receipt on completed session");

            let receipt_buf = client
                .download(&receipt_url)
                .map_err(|x| PokerError::BonsaiSdkError(x.to_string()))?;
            let receipt: Receipt = bincode::deserialize(&receipt_buf).unwrap();

            return Ok((receipt, session));
        }

        return Err(PokerError::BonsaiSdkError(format!(
            "Workflow exited: {} - | err: {}",
            res.status,
            res.error_msg.unwrap_or_default()
        )));
    }
}

#[cfg(test)]
mod test {
    use crate::stark::prove_bonsai;
    use poker_core::{
        mock_data::task0::mock_task0,
        task::{Task0, TaskCommit},
    };
    use poker_methods::POKER_METHOD_ID;
    use risc0_zkvm::serde::from_slice;
    use std::time::Instant;

    #[test]
    #[cfg(all(feature = "serialize0", feature = "deserialize0"))]
    fn bonsai_sdk_test() {
        dotenv::dotenv().ok();
        let task_bytes = mock_task0();
        let task: Task0 = from_slice(&task_bytes).unwrap();

        let start = Instant::now();
        let (receipt, _) = prove_bonsai(&task).unwrap();
        println!("Prover time: {:.2?}", start.elapsed());

        assert!(receipt.verify(POKER_METHOD_ID).is_ok());

        let commit: TaskCommit = receipt.journal.decode().unwrap();
        assert_eq!(commit.room_id, task.room_id);
        assert_eq!(commit.players_hand, task.players_hand);
        assert_eq!(commit.first_player, task.first_player);
    }
}
