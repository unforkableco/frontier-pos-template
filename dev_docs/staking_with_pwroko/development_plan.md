# Development Plan: Staking with pwROKO

## User Request Summary

The user requested a modification to the staking system to use the wrapped token `pwRoko` (managed by `pallet-pwroko`) instead of the native ROKO token (managed by `pallet-balances`) for all staking operations (bonding, rewards, slashing).

## Agent Analysis

This change requires modifying how `pallet-staking` is configured in the runtimes (testnet and mainnet) and potentially enhancing `pallet-pwroko` to support the necessary interactions (e.g., minting for rewards, burning for slashing, implementing standard fungible traits). Genesis configuration in the chain specifications must also be updated to reflect staking with `pwRoko`. This is a complex change impacting core economic functions of the chain.

## Relevant Files

*   `runtime/pallets/pwroko/src/lib.rs`: Core logic for pwRoko.
*   `runtime/pallets/pwroko/Cargo.toml`: Dependencies for pwRoko pallet.
*   `runtime/testnet/src/lib.rs`: Testnet runtime configuration.
*   `runtime/mainnet/src/lib.rs`: Mainnet runtime configuration.
*   `runtime/testnet/Cargo.toml`: Testnet runtime dependencies.
*   `runtime/mainnet/Cargo.toml`: Mainnet runtime dependencies.
*   `node/cli/src/chain_spec/testnet.rs`: Testnet genesis configuration.
*   `node/cli/src/chain_spec/mainnet.rs`: Mainnet genesis configuration.
*   Workspace `Cargo.toml`: Defines top-level dependencies like `pallet-staking`.

## Broad Steps

1.  **Enhance `pallet-pwroko`:** Implement necessary traits and functions.
2.  **Reconfigure `pallet-staking`:** Update runtime configurations.
3.  **Update Genesis Configuration:** Modify chain specifications.
4.  **Testing:** Implement unit and integration tests.
5.  **Benchmarking & Weights:** Generate and integrate weights.
6.  **Result Validation:** Create validation script and manual steps.

## Detailed Execution Plan

**Phase 1: Enhance `pallet-pwroko`**

1.  **Implement `fungible` Traits:**
    *   In `runtime/pallets/pwroko/src/lib.rs`, implement `frame_support::traits::tokens::fungible::Inspect` for `Pallet<T>`. This involves providing functions like `total_issuance()`, `minimum_balance()`, `balance()`, `reducible_balance()`, `can_deposit()`, `can_withdraw()`.
    *   Implement `frame_support::traits::tokens::fungible::Mutate` for `Pallet<T>`. This requires implementing `mint_into()` and `burn_from()`. Ensure these functions update `Balances` and `TotalSupply` correctly. Decide on handling the ROKO lock linkage for rewards/slashing (initial plan: mint/burn `pwRoko` independently of ROKO lock for these operations).
    *   Consider implementing `frame_support::traits::tokens::fungible::Balanced` if needed by `pallet-staking` for imbalances.
2.  **Add Internal Reward/Slash Functions:**
    *   Create internal functions (e.g., `do_reward_mint`, `do_slash_burn`) within `pallet-pwroko` called by the `Mutate` trait implementations.
3.  **Genesis Configuration:**
    *   Add `GenesisConfig` struct to `pallet-pwroko`.
    *   Define fields for `initial_balances: Vec<(T::AccountId, T::Balance)>`.
    *   Implement `#[pallet::genesis_build]` to populate `Balances` and `TotalSupply`, ensuring coordination with `pallet-balances` locks in the chain spec for the underlying ROKO.
4.  **Update `Cargo.toml`:** Ensure necessary features/versions.

**Phase 2: Reconfigure `pallet-staking` in Runtimes**

1.  **Modify `pallet_staking::Config`:**
    *   In `runtime/testnet/src/lib.rs` and `runtime/mainnet/src/lib.rs`:
        *   Inspect the `pallet_staking::Config` trait definition (Polkadot SDK v1.13.0).
        *   Identify the associated type for the staking currency/asset.
        *   Update the runtime implementations to use `pallet_pwroko` for this type (e.g., `type StakingCurrency = PwRoko;` bounded by appropriate `fungible` traits).
    *   Update `CurrencyToVote` to convert `pwRoko` balance to `u128`.
2.  **Update `construct_runtime!`:** Ensure `PwRoko` is included.

**Phase 3: Update Genesis Configuration**

1.  **Modify Chain Specs:**
    *   In `node/cli/src/chain_spec/testnet.rs` and `node/cli/src/chain_spec/mainnet.rs`:
        *   Update `staking` genesis: use `pwRoko` amounts for initial stakers.
        *   Add/update `pwRoko` genesis: set `initial_balances` matching the staking config.
        *   Update `balances` genesis: add corresponding ROKO `locks` for the `pwRoko` pallet's `LOCK_ID`.

**Phase 4: Testing**

1.  **Unit Tests (`pallet-pwroko`):** Test new `fungible` traits and genesis logic.
2.  **Integration Tests:** Create tests for the full staking lifecycle (bond, nominate, reward, unbond, slash) using `pwRoko`, verifying underlying ROKO locks.
3.  **Build & Run Tests:** Use `cargo build --features=testnet` and `cargo test --features=testnet`.

**Phase 5: Benchmarking & Weights**

1.  **Benchmark `pallet-pwroko`:** Benchmark new/changed functions.
2.  **Benchmark `pallet-staking`:** Re-benchmark staking extrinsics in the new runtime context.
3.  **Generate & Integrate:** Generate and add `weights.rs` files.

**Phase 6: Result Validation (Script & Manual)**

1.  **Create Validation Script (`dev_docs/agent_tasks/staking_with_pwroko/validate.sh`):** Script to build, run node, perform basic checks (genesis state, locks, maybe bond), and cleanup.
2.  **Manual Validation Steps:** Outline steps using PolkadotJS UI to verify genesis, locks, bonding, nominating, rewards (if possible), unbonding, slashing with `pwRoko`.

## Validation Plan

*(To be filled in last)* 