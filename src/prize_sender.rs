use std::time::Duration;

use rsnano_core::{
    work::{WorkPool, WorkPoolImpl, WorkThresholds},
    Account, Amount, Block, KeyPair, RawKey, StateBlock,
};
use rsnano_rpc_client::NanoRpcClient;
use rsnano_rpc_messages::{AccountInfoArgs, BlockSubTypeDto, ProcessArgs};
use tokio::task::spawn_blocking;

pub(crate) struct PrizeSender {
    sender_keys: KeyPair,
}

impl PrizeSender {
    pub(crate) fn new(prv_key: RawKey) -> Self {
        Self {
            sender_keys: KeyPair::from(prv_key),
        }
    }

    pub(crate) async fn send_prize(&self, destination: Account, amount: Amount) {
        println!("sender account: {}", self.sender_keys.account());
        let rpc = NanoRpcClient::new("http://[::1]:7076".parse().unwrap());
        let info = rpc
            .account_info(
                AccountInfoArgs::build(self.sender_keys.account())
                    .include_representative()
                    .finish(),
            )
            .await
            .unwrap();

        let work = spawn_blocking(move || {
            println!("starting with PoW generation");
            let work_pool =
                WorkPoolImpl::new(WorkThresholds::publish_full().clone(), 4, Duration::ZERO);
            let work = work_pool
                .generate(info.frontier.into(), work_pool.threshold_base())
                .unwrap();
            println!("PoW generation finished");
            work
        })
        .await
        .unwrap();

        let block = Block::State(StateBlock::new(
            self.sender_keys.account(),
            info.frontier,
            info.representative.unwrap().into(),
            info.balance - amount,
            destination.into(),
            &self.sender_keys,
            work,
        ));

        let args = ProcessArgs::build(block.json_representation())
            .subtype(BlockSubTypeDto::Send)
            .finish();

        println!("SENDING THIS:");
        println!("{}", serde_json::to_string_pretty(&args).unwrap());

        rpc.process(args).await.unwrap();
    }
}
