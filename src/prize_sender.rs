use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use log::info;
use rsnano_core::{Account, Amount, Block, PrivateKey, StateBlockArgs};
use rsnano_rpc_client::NanoRpcClient;
use rsnano_rpc_messages::{AccountInfoArgs, BlockSubTypeDto, ProcessArgs};
use rsnano_work::WorkPool;
use tokio::{task::spawn_blocking, time::sleep};

const MIN_DELAY: Duration = Duration::from_secs(15);

pub(crate) struct PrizeSender {
    sender_key: PrivateKey,
    work_pool: Arc<WorkPool>,
}

impl PrizeSender {
    pub(crate) fn new(sender_key: PrivateKey) -> Self {
        let work_pool = WorkPool::builder().gpu_only().finish();
        Self {
            sender_key,
            work_pool: work_pool.into(),
        }
    }

    pub(crate) async fn send_prize(
        &self,
        destination: Account,
        prize: Amount,
    ) -> anyhow::Result<()> {
        let rpc = NanoRpcClient::new("http://[::1]:7076".parse()?);
        let info = rpc
            .account_info(
                AccountInfoArgs::build(self.sender_key.account())
                    .include_representative()
                    .finish(),
            )
            .await?;

        let start = Instant::now();
        let work_pool = self.work_pool.clone();
        let work = spawn_blocking(move || {
            info!("Starting with PoW generation");
            let work = work_pool.generate_send(info.frontier.into()).unwrap();
            info!("PoW generation finished");
            work
        })
        .await?;

        let elapsed = start.elapsed();
        if elapsed < MIN_DELAY {
            sleep(MIN_DELAY - elapsed).await;
        }

        let block: Block = StateBlockArgs {
            key: &self.sender_key,
            previous: info.frontier,
            representative: info
                .representative
                .ok_or_else(|| anyhow!("no rep field!"))?
                .into(),
            balance: info.balance - prize,
            link: destination.into(),
            work,
        }
        .into();

        let args = ProcessArgs::build(block.json_representation())
            .subtype(BlockSubTypeDto::Send)
            .finish();

        rpc.process(args).await?;
        Ok(())
    }
}
