use anyhow::anyhow;
use log::info;
use rsnano_core::{
    work::{WorkPool, WorkPoolImpl, WorkThresholds},
    Account, Amount, Block, PrivateKey, StateBlockArgs,
};
use rsnano_rpc_client::NanoRpcClient;
use rsnano_rpc_messages::{AccountInfoArgs, BlockSubTypeDto, ProcessArgs};
use std::time::Duration;
use tokio::task::spawn_blocking;

pub(crate) struct PrizeSender {
    sender_key: PrivateKey,
}

impl PrizeSender {
    pub(crate) fn new(sender_key: PrivateKey) -> Self {
        Self { sender_key }
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

        let work = spawn_blocking(move || {
            info!("Starting with PoW generation");
            let work_pool =
                WorkPoolImpl::new(WorkThresholds::publish_full().clone(), 4, Duration::ZERO);
            let work = work_pool
                .generate(info.frontier.into(), work_pool.threshold_base())
                .unwrap();
            info!("PoW generation finished");
            work
        })
        .await?;

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
