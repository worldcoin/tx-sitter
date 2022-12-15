use crate::db::Database;
use crate::proto::sitter::{
    self,
    sitter_server::{Sitter, SitterServer},
    LookupTransactionReply, SendTransactionReply, SendTransactionRequest, StatusReply,
    StatusRequest, StatusTransactionReply, Txid,
};
use crate::types::TransactionRequest;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

pub struct SitterAPI {
    #[allow(dead_code)]
    db: Arc<Database>,
}

#[tonic::async_trait]
impl Sitter for SitterAPI {
    async fn status(
        &self,
        request: Request<StatusRequest>,
    ) -> Result<Response<StatusReply>, Status> {
        info!("received status request: {:?}", request);
        Ok(Response::new(StatusReply {
            status: sitter::Status::Normal.into(),
            active_upstreams: 0,
            pending_transactions: 0,
        }))
    }

    async fn send_transaction(
        &self,
        request: Request<SendTransactionRequest>,
    ) -> Result<Response<SendTransactionReply>, Status> {
        info!("received send transaction request: {:?}", request);

        let req = request.into_inner(); // consumes self, throws metadata away
        let _req = TransactionRequest::try_from(req).map_err(|e| {
            Status::invalid_argument(format!("could not parse transaction request: {}", e))
        })?;

        // I.  check whether req.id already exists in the database
        // II. validate the transaction:
        //   (a) does the requested sender exist?
        //   (b) does the requested sender have enough money?
        //   (c) if gas_limit was not specified return
        //       Status::unimplemented
        // III. hash the request to find the txid
        // IV.  save the transaction to our database
        // V.   return a successful response

        Err(Status::unimplemented("unimplemented"))
        // Ok(Response::new(SendTransactionReply {}))
    }

    async fn status_transaction(
        &self,
        request: Request<Txid>,
    ) -> Result<Response<StatusTransactionReply>, Status> {
        info!("received status transaction request: {:?}", request);

        Err(Status::unimplemented("unimplemented"))
    }

    async fn lookup_transaction(
        &self,
        request: Request<Txid>,
    ) -> Result<Response<LookupTransactionReply>, Status> {
        info!("received lookup transaction request: {:?}", request);

        Err(Status::unimplemented("unimplemented"))
    }
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("unknown tonic error")]
    TonicError(#[from] tonic::transport::Error),
}

pub async fn run_server(api_address: SocketAddr, db: Arc<Database>) -> Result<(), ServerError> {
    let api = SitterAPI { db };

    tokio::spawn(async move {
        Server::builder()
            .add_service(SitterServer::new(api))
            .serve(api_address)
            .await
            .unwrap();
    });

    info!(addr = api_address.to_string(), "api started");

    Ok(())
}
