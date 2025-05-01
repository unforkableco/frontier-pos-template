# Development Plan: Rename Native Coin to ROKO (18 Decimals)

This plan outlines the steps required to change the native coin symbol to ROKO and its decimal precision to 18.

## 1. Identify Pertinent Files

Based on the codebase structure and task requirements, the following files are likely involved:

*   `node/cli/src/chain_spec.rs`: Defines the genesis configuration, including initial token properties.
*   `runtime/common/src/constants.rs`: Likely contains constants related to currency, potentially including decimals.
*   `runtime/common/src/lib.rs`: The main library for the common runtime configuration. Might use or define token properties.
*   `runtime/mainnet/src/lib.rs`: Mainnet runtime implementation. Needs checks for token symbol/decimals usage.
*   `runtime/testnet/src/lib.rs`: Testnet runtime implementation. Needs checks for token symbol/decimals usage.
*   Potentially other files using the old symbol or assuming the old decimal precision (e.g., tests, UI elements if present).

## 2. Broad Steps

1.  Modify Runtime Constants: Update decimal precision constants if they exist.
2.  Modify Chain Specification: Update the token symbol and potentially properties in the chain spec generation code.
3.  Review Runtime Implementations: Ensure mainnet and testnet runtimes correctly use the new symbol and decimals.
4.  Review Tests: Update any tests that rely on the old symbol or decimals.
5.  Build and Test: Compile the node and perform validation.
6.  Result Validation: Verify the changes on a running node.

## 3. Detailed Execution Plan

### Step 3.1: Modify Runtime Constants

*   **File:** `runtime/common/src/constants.rs`
*   **Action:** Search for constants related to currency decimals (e.g., `DECIMALS`, `NATIVE_TOKEN_DECIMALS`). If found, update the value to `18`. If a constant for the unit/planck relationship exists (e.g., `DOLLARS`, `CENTS`), update it according to the new 18 decimal precision (1 ROKO = 10^18 Planck).

### Step 3.2: Modify Chain Specification

*   **File:** `node/cli/src/chain_spec.rs`
*   **Action:**
    *   Locate the function(s) responsible for generating the chain specification (e.g., `testnet_genesis`, `mainnet_genesis`, or similar helper functions).
    *   Find where the `properties` struct (usually a `Properties` type from `sp_core`) is populated.
    *   Change the `tokenSymbol` value from its current value (likely "UNIT" or similar) to `"ROKO"`.
    *   Change the `tokenDecimals` value to `18`.
    *   Review how initial balances are set (e.g., in the `balances` pallet configuration) to ensure they are still appropriate with 18 decimals. The raw values might need adjustment (e.g., multiply by 10^6 if changing from 12 to 18 decimals) to represent the same effective amount.

### Step 3.3: Review Runtime Implementations

*   **Files:** `runtime/mainnet/src/lib.rs`, `runtime/testnet/src/lib.rs`
*   **Action:**
    *   Search for any hardcoded instances of the old token symbol. Replace with "ROKO" if found.
    *   Search for any hardcoded assumptions about the number of decimals (e.g., formatting, calculations). Ensure they use the constant defined in `runtime/common/src/constants.rs` or are updated to 18.
    *   Pay attention to `CurrencyToVote` implementations or similar logic that might involve token amounts.

### Step 3.4: Review Tests

*   **Action:** Search the entire codebase (especially within `runtime/` and `node/` tests) for:
    *   The old token symbol (e.g., "UNIT"). Replace with "ROKO".
    *   Hardcoded numeric values that represented token amounts based on the old decimal precision. Adjust these values for 18 decimals (e.g., `1_000_000_000_000` might become `1_000_000_000_000_000_000`).

### Step 3.5: Build and Test

*   **Action:**
    *   Run `cargo check --all` to catch initial errors.
    *   Run `cargo test --all` to execute unit and integration tests. Fix any failures, likely related to Step 3.4.
    *   Build the node using the specified command: `cargo build --release --features=testnet` (and potentially `--features=mainnet` if needed).

### Step 3.6: Result Validation

*   **Manual Validation Steps:**
    1.  Start a local testnet node in dev mode: `./target/release/substrate --database auto --alice --dev`
    2.  Connect to the node using the Polkadot-JS Apps UI (or a similar tool).
    3.  **Check Token Symbol:** Verify that the UI displays "ROKO" as the native token symbol in account balances, transfer menus, etc.
    4.  **Check Decimals:**
        *   Observe the balance of an account (e.g., Alice).
        *   Perform a small transfer (e.g., 0.000000000000000001 ROKO, which is 1 Planck). Verify the transaction succeeds and the balance updates correctly, reflecting the 18-decimal precision.
        *   Attempt to transfer 1 ROKO. Verify the amount displayed and transferred corresponds to 10^18 Planck units.
    5.  Stop the node.

*   **Validation Script:** (Optional, but recommended by develop.md)
    *   A script could potentially use `curl` to query the RPC endpoint (`system_properties`) to check `tokenSymbol` and `tokenDecimals`.
    *   Automating balance checks and transfers via RPC is more complex but possible. For now, manual validation is sufficient. If requested later, a script `dev_docs/rename_native_coin_roko/validate_rename.sh` could be created.

## 4. User Approval

Present this plan to the user for review and approval before proceeding with code modifications. 