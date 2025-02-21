use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    cell::RefCell,
};

use actix_web::{error, web, HttpResponse};
use async_channel::{Receiver, Send, Sender};
use crossbeam::channel::{Sender as CBSender, Receiver as CBReceiver};
use serde::{Deserialize, Serialize};
use solana_sdk::hash::Hash; // keccak::Hash
use solana_sdk::transaction::Transaction;
use {
    crate::delegation_service::DelegationService,
    solana_sdk::pubkey::Pubkey,
    std::sync::{RwLock},
};

use crate::rollupdb::RollupDBMessage;

// message format to send found transaction from db to frontend
#[derive(Serialize, Deserialize)]
pub struct FrontendMessage {
    pub get_tx: Option<Hash>,
    pub transaction: Option<Transaction>,
}

// message format used to get transaction client
#[derive(Serialize, Deserialize, Debug)]
pub struct GetTransaction {
    pub get_tx: String,
}

// message format used to receive transactions from clients
#[derive(Serialize, Deserialize, Debug)]
pub struct RollupTransaction {
    pub sender: String,
    pub sol_transaction: Transaction,
    pub keypair_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", content = "data")]
pub enum TransactionResponse {
    Success { message: String },
    Error { message: String },
}

// Add thread local storage for keypair
thread_local! {
    static KEYPAIR_STORAGE: RefCell<Option<Vec<u8>>> = RefCell::new(None);
}

pub async fn submit_transaction(
    body: web::Json<RollupTransaction>,
    sequencer_sender: web::Data<CBSender<(Transaction, Vec<u8>)>>,
) -> actix_web::Result<HttpResponse> {
    // Send both transaction and keypair bytes
    sequencer_sender.send((body.sol_transaction.clone(), body.keypair_bytes.clone()))
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TransactionResponse::Success {
        message: "Transaction submitted".to_string()
    }))
}

pub async fn get_transaction(
    body: web::Json<GetTransaction>,
    sequencer_sender: web::Data<Sender<Transaction>>,
    rollupdb_sender: web::Data<Sender<RollupDBMessage>>,
    frontend_receiver: web::Data<Receiver<FrontendMessage>>,
) -> actix_web::Result<HttpResponse> {
    // Validate transaction structure with serialization in function signature
    log::info!("Requested transaction");
    log::info!("{body:?}");

    rollupdb_sender
        .send(RollupDBMessage {
            lock_accounts: None,
            add_new_data: None,
            add_processed_transaction: None,
            frontend_get_tx: Some(Hash::new(body.get_tx.as_bytes())),
            add_settle_proof: None,
            get_account: None,
        })
        .await
        .unwrap();

    if let Ok(frontend_message) = frontend_receiver.recv().await {
        return Ok(HttpResponse::Ok().json(RollupTransaction {
            sender: "Rollup RPC".into(),
            sol_transaction: frontend_message.transaction.unwrap(),
            keypair_bytes: Vec::new(),
        }));
        // Ok(HttpResponse::Ok().json(HashMap::from([("Transaction status", "requested")])))
    }

    Ok(HttpResponse::Ok().json(HashMap::from([("Transaction status", "requested")])))
}

pub async fn test() -> HttpResponse {
    log::info!("Test request");
    HttpResponse::Ok().json(HashMap::from([("test", "success")]))
}
