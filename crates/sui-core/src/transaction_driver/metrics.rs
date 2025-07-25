// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_with_registry, HistogramVec,
    IntCounter, Registry,
};

const FINALITY_LATENCY_SEC_BUCKETS: &[f64] = &[
    0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85,
    0.9, 0.95, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6,
    2.7, 2.8, 2.9, 3.0, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5,
    7.0, 7.5, 8.0, 8.5, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
    25.0,
];

#[derive(Clone)]
pub struct TransactionDriverMetrics {
    pub(crate) settlement_finality_latency: HistogramVec,
    pub(crate) submit_transaction_success: IntCounter,
    pub(crate) submit_transaction_error: IntCounter,
    pub(crate) executed_transactions: IntCounter,
    pub(crate) rejection_acks: IntCounter,
    pub(crate) expiration_acks: IntCounter,
    pub(crate) effects_digest_mismatches: IntCounter,
}

impl TransactionDriverMetrics {
    pub fn new(registry: &Registry) -> Self {
        Self {
            settlement_finality_latency: register_histogram_vec_with_registry!(
                "transaction_driver_settlement_finality_latency",
                "Settlement finality latency observed from transaction driver",
                &["tx_type"],
                FINALITY_LATENCY_SEC_BUCKETS.to_vec(),
                registry,
            )
            .unwrap(),
            submit_transaction_success: register_int_counter_with_registry!(
                "transaction_driver_submit_transaction_success",
                "Number of transactions successfully submitted",
                registry,
            )
            .unwrap(),
            submit_transaction_error: register_int_counter_with_registry!(
                "transaction_driver_submit_transaction_error",
                "Number of transactions unsuccessfully submitted",
                registry,
            )
            .unwrap(),
            executed_transactions: register_int_counter_with_registry!(
                "transaction_driver_executed_transactions",
                "Number of transactions executed observed by the transaction driver",
                registry,
            )
            .unwrap(),
            rejection_acks: register_int_counter_with_registry!(
                "transaction_driver_rejected_acks",
                "Number of rejection acknowledgments observed by the transaction driver",
                registry,
            )
            .unwrap(),
            expiration_acks: register_int_counter_with_registry!(
                "transaction_driver_expiration_acks",
                "Number of expiration acknowledgments observed by the transaction driver",
                registry,
            )
            .unwrap(),
            effects_digest_mismatches: register_int_counter_with_registry!(
                "transaction_driver_effects_digest_mismatches",
                "Number of effects digest mismatches detected by the transaction driver",
                registry,
            )
            .unwrap(),
        }
    }

    pub fn new_for_tests() -> Self {
        let registry = Registry::new();
        Self::new(&registry)
    }
}
