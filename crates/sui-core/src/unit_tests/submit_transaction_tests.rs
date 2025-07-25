// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::sync::Arc;

use consensus_core::BlockStatus;
use consensus_types::block::BlockRef;
use fastcrypto::traits::KeyPair;
use sui_test_transaction_builder::TestTransactionBuilder;
use sui_types::base_types::{random_object_ref, ObjectRef, SuiAddress};
use sui_types::crypto::{get_account_key_pair, AccountKeyPair};
use sui_types::effects::TransactionEffectsAPI as _;
use sui_types::executable_transaction::VerifiedExecutableTransaction;
use sui_types::message_envelope::Message as _;
use sui_types::messages_grpc::RawSubmitTxRequest;
use sui_types::object::Object;
use sui_types::transaction::{
    Transaction, TransactionDataAPI, TransactionExpiration, VerifiedTransaction,
};
use sui_types::utils::to_sender_signed_transaction;

use crate::authority::test_authority_builder::TestAuthorityBuilder;
use crate::authority::{AuthorityState, ExecutionEnv};
use crate::authority_client::{AuthorityAPI, NetworkAuthorityClient};
use crate::authority_server::AuthorityServer;
use crate::consensus_adapter::consensus_tests::make_consensus_adapter_for_test;
use crate::execution_scheduler::SchedulingSource;
use crate::mock_consensus::with_block_status;
use crate::transaction_driver::SubmitTxResponse;

use super::AuthorityServerHandle;

struct TestContext {
    state: Arc<AuthorityState>,
    _server_handle: AuthorityServerHandle,
    client: NetworkAuthorityClient,
    sender: SuiAddress,
    keypair: AccountKeyPair,
    gas_object_ref: ObjectRef,
}

impl TestContext {
    async fn new() -> Self {
        telemetry_subscribers::init_for_testing();
        let (sender, keypair) = get_account_key_pair();
        let gas_object = Object::with_owner_for_testing(sender);
        let gas_object_ref = gas_object.compute_object_reference();
        let authority = TestAuthorityBuilder::new()
            .with_starting_objects(&[gas_object])
            .build()
            .await;

        // Create a server with mocked consensus.
        // This ensures transactions submitted to consensus will get processed.
        let adapter = make_consensus_adapter_for_test(
            authority.clone(),
            HashSet::new(),
            true,
            vec![
                with_block_status(BlockStatus::Sequenced(BlockRef::MIN)),
                with_block_status(BlockStatus::Sequenced(BlockRef::MIN)),
            ],
        );
        let server =
            AuthorityServer::new_for_test_with_consensus_adapter(authority.clone(), adapter);
        let _metrics = server.metrics.clone();
        let server_handle = server.spawn_for_test().await.unwrap();
        let client = NetworkAuthorityClient::connect(
            server_handle.address(),
            authority.config.network_key_pair().public().to_owned(),
        )
        .await
        .unwrap();

        Self {
            state: authority,
            _server_handle: server_handle,
            client,
            sender,
            keypair,
            gas_object_ref,
        }
    }

    fn build_test_transaction(&self) -> Transaction {
        let tx_data = TestTransactionBuilder::new(
            self.sender,
            self.gas_object_ref,
            self.state.reference_gas_price_for_testing().unwrap(),
        )
        .transfer_sui(None, self.sender)
        .build();
        to_sender_signed_transaction(tx_data, &self.keypair)
    }

    fn build_submit_request(&self, transaction: Transaction) -> RawSubmitTxRequest {
        RawSubmitTxRequest {
            transaction: bcs::to_bytes(&transaction).unwrap().into(),
        }
    }
}

#[tokio::test]
async fn test_submit_transaction_success() {
    let test_context = TestContext::new().await;

    let transaction = test_context.build_test_transaction();
    let request = test_context.build_submit_request(transaction);

    let response = test_context
        .client
        .submit_transaction(request, None)
        .await
        .unwrap();

    // Verify we got a consensus position back
    let response: SubmitTxResponse = response.try_into().unwrap();
    match response {
        SubmitTxResponse::Submitted { consensus_position } => {
            assert_eq!(consensus_position.index, 0);
        }
        _ => panic!("Expected Submitted response"),
    };
}

#[tokio::test]
async fn test_submit_transaction_invalid_transaction() {
    let test_context = TestContext::new().await;

    // Create an invalid request with malformed transaction bytes
    let request = RawSubmitTxRequest {
        transaction: vec![0xFF, 0xFF, 0xFF].into(),
    };

    let response = test_context.client.submit_transaction(request, None).await;

    assert!(response.is_err());
}

// test transaction submission after already executed.
#[tokio::test]
async fn test_submit_transaction_already_executed() {
    let test_context = TestContext::new().await;

    let transaction = test_context.build_test_transaction();
    let request = test_context.build_submit_request(transaction.clone());

    let epoch_store = test_context.state.epoch_store_for_testing();
    let verified_transaction = VerifiedExecutableTransaction::new_from_checkpoint(
        VerifiedTransaction::new_unchecked(transaction),
        epoch_store.epoch(),
        1,
    );
    test_context
        .state
        .try_execute_immediately(
            &verified_transaction,
            // Fastpath execution will only put outputs in a temporary cache,
            // and the object changes in this transaction are not yet committed.
            ExecutionEnv::new().with_scheduling_source(SchedulingSource::MysticetiFastPath),
            &epoch_store,
        )
        .await
        .unwrap();

    // Submit the same transaction that has already been fastpath executed.
    let response1 = test_context
        .client
        .submit_transaction(request.clone(), None)
        .await
        .unwrap();

    // Verify we still got a consensus position back, because the transaction has not been committed yet,
    // so we can still sign the same transaction.
    let response1: SubmitTxResponse = response1.try_into().unwrap();
    match response1 {
        SubmitTxResponse::Submitted { consensus_position } => {
            assert_eq!(consensus_position.index, 0);
        }
        _ => panic!("Expected Submitted response"),
    };

    // Execute it again through non-fastpath, which will commit the object changes.
    test_context
        .state
        .try_execute_immediately(
            &verified_transaction,
            ExecutionEnv::new().with_scheduling_source(SchedulingSource::NonFastPath),
            &epoch_store,
        )
        .await
        .unwrap();

    // Submit the same transaction again.
    let response2 = test_context
        .client
        .submit_transaction(request, None)
        .await
        .unwrap();
    // Verify we got the full effects back.
    let response2: SubmitTxResponse = response2.try_into().unwrap();
    match response2 {
        SubmitTxResponse::Executed {
            effects_digest,
            details,
        } => {
            let details = details.unwrap();
            assert_eq!(effects_digest, details.effects.digest());
            assert_eq!(
                verified_transaction.digest(),
                details.effects.transaction_digest()
            );
        }
        _ => panic!("Expected Executed response"),
    };
}

#[tokio::test]
async fn test_submit_transaction_wrong_epoch() {
    let test_context = TestContext::new().await;
    test_context.state.reconfigure_for_testing().await;

    // Build a transaction with wrong epoch
    let tx_data = TestTransactionBuilder::new(
        test_context.sender,
        test_context.gas_object_ref,
        test_context
            .state
            .reference_gas_price_for_testing()
            .unwrap(),
    )
    .transfer_sui(None, test_context.sender)
    .build();

    // Manually set wrong epoch
    let mut tx_data = tx_data;
    *tx_data.expiration_mut_for_testing() = TransactionExpiration::Epoch(0);

    let transaction = to_sender_signed_transaction(tx_data, &test_context.keypair);
    let request = test_context.build_submit_request(transaction);

    let response = test_context.client.submit_transaction(request, None).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_submit_transaction_signature_verification_failure() {
    let test_context = TestContext::new().await;

    let tx_data = TestTransactionBuilder::new(
        test_context.sender,
        test_context.gas_object_ref,
        test_context
            .state
            .reference_gas_price_for_testing()
            .unwrap(),
    )
    .transfer_sui(None, test_context.sender)
    .build();

    // Sign with a different keypair to cause signature verification failure
    let (_, wrong_keypair) = get_account_key_pair();
    let transaction = to_sender_signed_transaction(tx_data, &wrong_keypair);
    let request = test_context.build_submit_request(transaction);

    let response = test_context.client.submit_transaction(request, None).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_submit_transaction_gas_object_validation() {
    let test_context = TestContext::new().await;

    // Build a transaction with an invalid gas object reference
    let invalid_gas_ref = random_object_ref();
    let tx_data = TestTransactionBuilder::new(
        test_context.sender,
        invalid_gas_ref,
        test_context
            .state
            .reference_gas_price_for_testing()
            .unwrap(),
    )
    .transfer_sui(None, test_context.sender)
    .build();

    let transaction = to_sender_signed_transaction(tx_data, &test_context.keypair);
    let request = test_context.build_submit_request(transaction);

    let response = test_context.client.submit_transaction(request, None).await;
    assert!(response.is_err());
}
