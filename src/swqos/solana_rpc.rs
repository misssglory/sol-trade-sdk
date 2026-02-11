use std::{sync::Arc, time::Instant};

use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_commitment_config::CommitmentLevel;
use solana_sdk::message::VersionedMessage;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::UiTransactionEncoding;
use tracing::{error, info};

use crate::swqos::SwqosClientTrait;
use crate::{
    common::SolanaRpcClient,
    swqos::{common::poll_transaction_confirmation, SwqosType, TradeType},
};
use anyhow::Result;

#[derive(Clone)]
pub struct SolRpcClient {
    pub rpc_client: Arc<SolanaRpcClient>,
}

fn print_versioned_transaction_instructions(tx: &VersionedTransaction) {
    match &tx.message {
        VersionedMessage::V0(message) => {
            log::error!("Transaction Version: V0");
            for (i, instruction) in message.instructions.iter().enumerate() {
                // Get the program_id from the account keys using the program_id_index
                let program_id = &message.account_keys[instruction.program_id_index as usize];
                
                log::error!("Instruction {}:", i);
                log::error!("  Program ID: {}", program_id);
                log::error!("  Account Indices: {:?}", instruction.accounts);
                log::error!("  Data (bytes): {:?}", instruction.data); 
            }
        },
        VersionedMessage::Legacy(message) => {
            log::error!("Transaction Version: Legacy");
            for (i, instruction) in message.instructions.iter().enumerate() {
                // In legacy messages, the program_id is directly available
                let program_id = &message.account_keys[instruction.program_id_index as usize];

                log::error!("Instruction {}:", i);
                log::error!("  Program ID: {}", program_id);
                log::error!("  Account Indices: {:?}", instruction.accounts);
                log::error!("  Data (bytes): {:?}", instruction.data);
            }
        },
    }
}

#[async_trait::async_trait]
impl SwqosClientTrait for SolRpcClient {
    async fn send_transaction(
        &self,
        trade_type: TradeType,
        transaction: &VersionedTransaction,
        wait_confirmation: bool,
    ) -> Result<()> {
        let signature = self
            .rpc_client
            .send_transaction_with_config(
                transaction,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: Some(CommitmentLevel::Processed),
                    encoding: Some(UiTransactionEncoding::Base64),
                    max_retries: Some(3),
                    min_context_slot: Some(0),
                },
            )
            .await?;

        let start_time = Instant::now();
        match poll_transaction_confirmation(&self.rpc_client, signature, wait_confirmation).await {
            Ok(_) => (),
            Err(e) => {
                log::error!(" signature: {:?}", signature);
                log::error!(" [rpc] {} confirmation failed: {:?}", trade_type, start_time.elapsed());
                // log::error!("{}", transaction);
                print_versioned_transaction_instructions(transaction);
                log::error!("RPC transaction error: {}", e);
                return Err(e);
            }
        }
        if wait_confirmation {
            log::info!(" signature: {:?}", signature);
            log::info!(" [rpc] {} confirmed: {:?}", trade_type, start_time.elapsed());
        }

        Ok(())
    }

    async fn send_transactions(
        &self,
        trade_type: TradeType,
        transactions: &Vec<VersionedTransaction>,
        wait_confirmation: bool,
    ) -> Result<()> {
        for transaction in transactions {
            self.send_transaction(trade_type, transaction, wait_confirmation).await?;
        }
        Ok(())
    }

    fn get_tip_account(&self) -> Result<String> {
        Ok("".to_string())
    }

    fn get_swqos_type(&self) -> SwqosType {
        SwqosType::Default
    }
}

impl SolRpcClient {
    pub fn new(rpc_client: Arc<SolanaRpcClient>) -> Self {
        Self { rpc_client }
    }
}
