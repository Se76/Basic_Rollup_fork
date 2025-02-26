//! A helper to initialize Solana SVM API's `TransactionBatchProcessor`.

use {
    solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1, solana_compute_budget::compute_budget::ComputeBudget, solana_program_runtime::loaded_programs::{BlockRelation, ForkGraph, LoadProgramMetrics, ProgramCacheEntry}, solana_sdk::{account::ReadableAccount, clock::Slot, feature_set::FeatureSet, pubkey::Pubkey, transaction}, solana_svm::{
        account_loader::CheckedTransactionDetails,
        transaction_processing_callback::TransactionProcessingCallback,
        transaction_processor::TransactionBatchProcessor,
    }, solana_system_program::system_processor, spl_token_2022, std::sync::{Arc, RwLock},
    solana_client::rpc_client::RpcClient,
};

/// In order to use the `TransactionBatchProcessor`, another trait - Solana
/// Program Runtime's `ForkGraph` - must be implemented, to tell the batch
/// processor how to work across forks.
///
/// Since PayTube doesn't use slots or forks, this implementation is mocked.
pub(crate) struct RollupForkGraph {}

impl ForkGraph for RollupForkGraph {
    fn relationship(&self, _a: Slot, _b: Slot) -> BlockRelation {
        BlockRelation::Unknown
    }
}

/// This function encapsulates some initial setup required to tweak the
/// `TransactionBatchProcessor` for use within PayTube.
///
/// We're simply configuring the mocked fork graph on the SVM API's program
/// cache, then adding the System program to the processor's builtins.
pub(crate) fn create_transaction_batch_processor<CB: TransactionProcessingCallback>(
    callbacks: &CB,
    feature_set: &FeatureSet,
    compute_budget: &ComputeBudget,
    fork_graph: Arc<RwLock<RollupForkGraph>>,
    // needed_programs: Vec<Pubkey>,
) -> TransactionBatchProcessor<RollupForkGraph> {
    // Create a new transaction batch processor.
    //
    // We're going to use slot 1 specifically because any programs we add will
    // be deployed in slot 0, and they are delayed visibility until the next
    // slot (1).
    // This includes programs owned by BPF Loader v2, which are automatically
    // marked as "depoyed" in slot 0.
    // See `solana_svm::program_loader::program_with_pubkey` for more
    // details.
    let processor = TransactionBatchProcessor::<RollupForkGraph>::new_uninitialized(
        /* slot */ 1,
        /* epoch */ 1,
        // Arc::downgrade(&fork_graph),
        // Some(Arc::new(
        //     create_program_runtime_environment_v1(feature_set, compute_budget, false, false)
        //         .unwrap(),
        // )),
        // None,
    );

    let rpc_client_temp = RpcClient::new("https://api.devnet.solana.com".to_string());

    processor.program_cache.write().unwrap().set_fork_graph(Arc::downgrade(&fork_graph));
    {
    let mut cache = processor.program_cache.write().unwrap();
        cache.environments.program_runtime_v1 = Arc::new(create_program_runtime_environment_v1(feature_set, compute_budget, false, false).unwrap());
        // Add the SPL Token program to the cache.
        if let Some(account) = callbacks.get_account_shared_data(&spl_token::id()) {
            let elf_bytes = account.data();
            let program_runtime_environment = cache.environments.program_runtime_v1.clone();
            cache.assign_program(
                spl_token::id(), 
                Arc::new(
                    ProgramCacheEntry::new(
                        &solana_sdk::bpf_loader::id(), 
                        program_runtime_environment, 
                        1, 
                        1, 
                        elf_bytes, 
                        elf_bytes.len(), 
                        &mut LoadProgramMetrics::default(),
                    ).unwrap()
                )
            );
        }
        if let Some(account) = callbacks.get_account_shared_data(&spl_token_2022::id()) {
            // let pubkey_to_the_elf_bytes = account.owner();
            // let account_holding_elf_bytes = rpc_client_temp.get_account(&pubkey_to_the_elf_bytes).unwrap();
            // let elf_bytes = account_holding_elf_bytes.data();    
            let elf_bytes = account.data();
            let program_runtime_environment = cache.environments.program_runtime_v1.clone();
            cache.assign_program(
                spl_token_2022::id(), 
                Arc::new(
                    ProgramCacheEntry::new(
                        &solana_sdk::bpf_loader_upgradeable::id(), 
                        program_runtime_environment, 
                        1, 
                        1, 
                        elf_bytes, 
                        elf_bytes.len(), 
                        &mut LoadProgramMetrics::default(),
                    ).unwrap()
                )
            );
        }
    }
    processor.prepare_program_cache_for_upcoming_feature_set(callbacks, feature_set, compute_budget, 1, 50);

    // processor.prepare_program_cache_for_upcoming_feature_set(callbacks, upcoming_feature_set, compute_budget, slot_index, slots_in_epoch);

    // Add the system program builtin.
    processor.add_builtin(
        callbacks,
        solana_system_program::id(),
        "system_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"system_program".len(),
            system_processor::Entrypoint::vm,
        ),
    );

    // processor.program_cache.read().unwrap().programs_to_recompile

    // Add the BPF Loader v2 builtin, for the SPL Token program.
    processor.add_builtin(
        callbacks,
        solana_sdk::bpf_loader::id(),
        "solana_bpf_loader_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"solana_bpf_loader_program".len(),
            solana_bpf_loader_program::Entrypoint::vm,
        ),
    );

    // processor.add_builtin(
    //     callbacks,
    //     solana_inline_spl::token::id(),
    //     "token_program",
    //     ProgramCacheEntry::new_builtin(
    //         0,
    //         b"token_program".len(),
    //         spl_token::processor::Processor::process?????????, // solana_inline_spl::token::????

    //     )
    // );

    // Adding any needed programs to the processor.


    processor
}

/// This function is also a mock. In the Agave validator, the bank pre-checks
/// transactions before providing them to the SVM API. We mock this step in
/// PayTube, since we don't need to perform such pre-checks.
pub(crate) fn get_transaction_check_results(
    len: usize,
    lamports_per_signature: u64,
) -> Vec<transaction::Result<CheckedTransactionDetails>> {
    vec![
        transaction::Result::Ok(CheckedTransactionDetails {
            nonce: None,
            lamports_per_signature,
        });
        len
    ]
}
