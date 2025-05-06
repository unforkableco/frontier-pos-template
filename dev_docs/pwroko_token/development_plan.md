# Development Plan: pwRoko Wrapped Token

## Pertinent Files

*   **Runtime Pallets:**
    *   A new pallet (e.g., `runtime/pallets/pwroko/src/lib.rs`) will need to be created to manage the locking of `ROKO` and the minting/burning of `pwRoko`.
    *   `runtime/pallets/balances/src/lib.rs` (or similar standard balances pallet): For interacting with the native `ROKO` token.
    *   `runtime/pallets/evm/src/lib.rs` (Frontier's EVM pallet): For handling EVM interactions.
    *   `runtime/pallets/ethereum/src/lib.rs` (Frontier's Ethereum pallet): For mapping Substrate assets to ERC20 representations.
*   **Runtime Configuration:**
    *   `runtime/common/src/lib.rs`: Potentially for shared types or configurations.
    *   `runtime/mainnet/src/lib.rs`: To integrate the new pallet and configure it for the mainnet runtime.
    *   `runtime/testnet/src/lib.rs`: To integrate the new pallet and configure it for the testnet runtime.
*   **Node/Chain Specification:**
    *   `node/cli/src/chain_spec.rs`: Might need updates if genesis configuration for the new pallet is required.
*   **Tests:**
    *   New test file(s) within the `pwroko` pallet directory (e.g., `runtime/pallets/pwroko/src/tests.rs`).
    *   Possibly integration tests involving EVM interactions.

## Broad Steps

1.  **Create the `pwroko` Pallet:** Define storage, events, errors, and extrinsics for locking `ROKO`, unlocking `ROKO`, minting `pwRoko`, and burning `pwRoko`. Implement basic ERC20-like functionality (balance, transfer, approve, allowance, transferFrom).
2.  **Integrate `pwroko` Pallet into Runtimes:** Add the `pwroko` pallet to the `construct_runtime!` macro and configure its `Config` trait implementation in both `mainnet` and `testnet` runtimes.
3.  **Implement EVM/ERC20 Interface:** Configure Frontier's `pallet-ethereum` or similar mechanisms to map the `pwRoko` asset managed by the new pallet to an ERC20 contract address within the EVM, allowing interaction via standard Ethereum tools/wallets.
4.  **Add Unit and Integration Tests:** Write tests to verify the core logic of the `pwroko` pallet (locking/unlocking, minting/burning, transfers) and its interaction with the EVM as an ERC20 token.
5.  **Result Validation:** Define steps for manual validation and create an automated validation script.

## Detailed Execution Plan

### Step 1: Create the `pwroko` Pallet

*   **Location:** `runtime/pallets/pwroko`
*   **Files:**
    *   `Cargo.toml`: Define dependencies (e.g., `frame-support`, `frame-system`, `pallet-balances`, potentially `sp-runtime`, `sp-std`, Frontier pallets if needed directly).
    *   `src/lib.rs`:
        *   Define `Config` trait: Include necessary associated types like `RuntimeEvent`, `Currency` (for `ROKO`), `AssetId` (if using a generic asset framework, though maybe not needed if it's just one token), `Balance`, etc. Constraint `Currency` to use the native `ROKO`.
        *   Define `Pallet` struct.
        *   **Storage:**
            *   `LockedBalances`: `StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance>` - Stores the amount of `ROKO` locked by each account.
            *   `TotalSupply`: `StorageValue<_, T::Balance>` - Stores the total supply of `pwRoko`.
            *   `Balances`: `StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance>` - Stores `pwRoko` balance for each account (ERC20 `balanceOf`).
            *   `Allowances`: `StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, T::AccountId, T::Balance>` - Stores ERC20 allowances (`allowance`).
        *   **Events:**
            *   `Locked(who, amount)`: Emitted when `ROKO` is locked.
            *   `Unlocked(who, amount)`: Emitted when `ROKO` is unlocked.
            *   `Minted(to, amount)`: Emitted when `pwRoko` is minted.
            *   `Burned(from, amount)`: Emitted when `pwRoko` is burned.
            *   `Transfer(from, to, amount)`: Emitted on `pwRoko` transfer (ERC20 `Transfer`).
            *   `Approval(owner, spender, amount)`: Emitted on `pwRoko` approval (ERC20 `Approval`).
        *   **Errors:** `InsufficientLockedBalance`, `Insufficient_pwRoko_Balance`, `AmountZero`, `Overflow`, `InsufficientAllowance`.
        *   **Extrinsics:**
            *   `lock(amount)`: Locks `amount` of `ROKO` from the caller, reserves it using `pallet-balances`, updates `LockedBalances`, mints an equivalent `amount` of `pwRoko` to the caller, updates `Balances` and `TotalSupply`, emits `Locked` and `Minted` events.
            *   `unlock(amount)`: Burns `amount` of `pwRoko` from the caller, updates `Balances` and `TotalSupply`, unreserves the equivalent `amount` of `ROKO` using `pallet-balances`, updates `LockedBalances`, emits `Burned` and `Unlocked` events. Requires sufficient `pwRoko` balance and corresponding locked `ROKO`.
            *   `transfer(to, amount)`: Transfers `amount` of `pwRoko` from caller to `to`. Implements ERC20 `transfer`. Updates `Balances`, emits `Transfer`.
            *   `approve(spender, amount)`: Approves `spender` to withdraw `amount` of `pwRoko` from the caller. Implements ERC20 `approve`. Updates `Allowances`, emits `Approval`.
            *   `transfer_from(from, to, amount)`: Transfers `amount` of `pwRoko` from `from` to `to` using the allowance set by `approve`. Implements ERC20 `transferFrom`. Updates `Balances` and `Allowances`, emits `Transfer`.
        *   **Public functions (for ERC20 interface via EVM mapping):**
            *   `balance_of(who)`: Returns `pwRoko` balance.
            *   `allowance(owner, spender)`: Returns allowance.
            *   `total_supply()`: Returns total `pwRoko` supply.
            *   Implement helper functions for minting, burning, and transferring `pwRoko` internally.

### Step 2: Integrate `pwroko` Pallet into Runtimes

*   **Files:** `runtime/mainnet/src/lib.rs`, `runtime/testnet/src/lib.rs`, potentially `runtime/common/src/lib.rs`.
*   Add `pwroko` pallet crate as a dependency in `runtime/<mainnet|testnet>/Cargo.toml`.
*   In `runtime/<mainnet|testnet>/src/lib.rs`:
    *   Declare the pallet: `pub use pwroko;`
    *   Implement the `pwroko::Config` trait for the runtime. Map associated types correctly (e.g., `RuntimeEvent`, `Currency` to the configured `pallet-balances` instance, `Balance` type, etc.).
    *   Add the `pwRoko` pallet to the `construct_runtime!` macro. Assign a unique `PalletIndex`.
*   If common types are needed (e.g., `AssetId` although maybe not here), define them in `runtime/common/src/lib.rs`.

### Step 3: Implement EVM/ERC20 Interface

*   **Files:** `runtime/mainnet/src/lib.rs`, `runtime/testnet/src/lib.rs`.
*   **Leverage Frontier's `pallet-evm` and `pallet-ethereum`:**
    *   Ensure `pallet-evm` is configured correctly to execute EVM transactions.
    *   Configure `pallet-ethereum`'s `EnsureAddressOrigin` or similar traits if needed for mapping Substrate accounts to Ethereum addresses.
    *   **Precompiles:** The most common way to expose Substrate pallet functionality (like ERC20 tokens) to the EVM is through precompiled contracts.
        *   Investigate if Frontier provides a standard ERC20 precompile that can be configured to wrap a Substrate asset/balance type managed by our `pwroko` pallet. This is the ideal scenario. We would need to:
            *   Assign a specific precompile address (e.g., within the `0x00...01` to `0x00...ff` range).
            *   Configure the precompile set in the runtime's `pallet-evm` configuration (`Runner::config().precompiles`).
            *   The precompile would need to call the appropriate functions in our `pwroko` pallet (`balance_of`, `transfer`, `approve`, `transfer_from`, `total_supply`, `allowance`).
        *   If a standard precompile isn't suitable or available, a custom precompile might need to be implemented that directly interacts with the `pwroko` pallet's storage and functions. This is more complex.
    *   **Alternative (Less Common):** Use `pallet-assets` along with Frontier's features that might automatically expose assets as ERC20 tokens. However, since we need custom lock/unlock logic tied to the native currency, a dedicated pallet (`pwroko`) with a precompile seems more appropriate than just using `pallet-assets`.
*   Verify that EVM calls to the assigned ERC20 address correctly interact with the `pwroko` pallet's state (`Balances`, `Allowances`, `TotalSupply`).

### Step 4: Add Unit and Integration Tests

*   **Files:** `runtime/pallets/pwroko/src/tests.rs`, potentially new integration test files.
*   **Unit Tests (`tests.rs`):**
    *   Use the `frame_support::construct_runtime!` mock runtime setup.
    *   Test `lock`: Correct `ROKO` reservation, `pwRoko` minting, event emission, state updates. Edge cases: locking zero, locking more than available `ROKO`.
    *   Test `unlock`: Correct `pwRoko` burning, `ROKO` un-reservation, event emission, state updates. Edge cases: unlocking zero, unlocking more `pwRoko` than owned, unlocking without sufficient locked `ROKO`.
    *   Test `transfer`: Correct balance updates, event emission. Edge cases: transfer zero, transfer more than balance, transfer to self.
    *   Test `approve`: Correct allowance updates, event emission.
    *   Test `transfer_from`: Correct balance and allowance updates, event emission. Edge cases: transfer zero, transfer more than allowed, transfer more than sender's balance.
    *   Test basic ERC20 view functions (`balance_of`, `allowance`, `total_supply`) if implemented directly, although these are often implicitly tested via extrinsics.
*   **Integration Tests (Potentially in `node/cli/tests` or similar):**
    *   Set up a test environment using `substrate-cli-test-utils` or similar.
    *   Test locking `ROKO` via extrinsic and then verifying `pwRoko` balance via EVM call (using the precompile address).
    *   Test transferring `pwRoko` via EVM call and verifying balances via both EVM and Substrate state queries.
    *   Test approving `pwRoko` via EVM call and then using `transferFrom` via EVM call.
    *   Test burning `pwRoko` via extrinsic (`unlock`) and verifying `ROKO` is unreserved and `pwRoko` balance is zero via EVM.

### Step 5: Result Validation

*   **Manual Validation Steps:**
    1.  Start a local testnet node (`cargo run --features=testnet -- --dev --tmp`).
    2.  Use PolkadotJS Apps connected to the local node:
        *   Check initial `ROKO` balance of a dev account (e.g., Alice).
        *   Use `Developer -> Extrinsics` to call `pwroko.lock` with a specific amount. Verify success and check events.
        *   Verify `ROKO` balance decreased by the locked amount + fees, and the reserved balance increased.
        *   Use `Developer -> Chain State` to query `pwroko.balances` and `pwroko.lockedBalances` for Alice.
    3.  Use MetaMask connected to the local node's EVM RPC endpoint (e.g., `http://localhost:8545`):
        *   Add `pwRoko` as a custom token using the precompile address determined in Step 3.
        *   Verify the `pwRoko` balance matches the amount minted in step 2.
        *   Transfer some `pwRoko` from Alice to Bob via MetaMask. Verify success.
        *   Check Alice's and Bob's `pwRoko` balances in MetaMask and via PolkadotJS Chain State (`pwroko.balances`).
        *   Use PolkadotJS Apps (`Developer -> Extrinsics`) to call `pwroko.unlock` for Alice with the remaining `pwRoko` amount. Verify success and check events.
        *   Verify Alice's `ROKO` balance is restored (minus fees) and reserved balance is zero.
        *   Verify Alice's `pwRoko` balance is zero in both MetaMask and PolkadotJS Chain State.
*   **Automated Validation Script (`dev_docs/pwroko_token/validate_pwroko.sh`):**
    *   Script Name: `validate_pwroko.sh`
    *   Format: `.sh`
    *   Functionality:
        1.  (Optional) Build the node: `cargo build --release --features=testnet`
        2.  Start node in background: `./target/release/substrate --database auto --alice --dev --tmp &> node.log &` (Save PID)
        3.  Wait for node to be ready (e.g., check RPC endpoint).
        4.  Use `curl` or a command-line tool (`wscat`, JS script with PolkadotJS API, etc.) to:
            *   Query initial `ROKO` balance for Alice.
            *   Submit `pwroko.lock` extrinsic.
            *   Query `ROKO` balance/reserved state.
            *   Query `pwroko.balances` and `pwroko.lockedBalances` via RPC.
        5.  Use `cast` (from Foundry) or a similar tool (`web3.js`/`ethers.js` script) to interact with the EVM:
            *   Query `pwRoko` balance (ERC20 `balanceOf`) at the precompile address for Alice.
            *   Perform an ERC20 `transfer` of `pwRoko` from Alice to Bob.
            *   Query `pwRoko` balances for Alice and Bob.
        6.  Use RPC call to submit `pwroko.unlock` extrinsic for Alice.
        7.  Query final `ROKO` balance/reserved state for Alice.
        8.  Query final `pwRoko` balance for Alice via EVM.
        9.  Compare results against expected values. Print detailed PASS/FAIL messages for each step.
        10. Terminate the background node process using the saved PID.
        11. Output relevant parts of `node.log` if checks fail. 

## Usage Guide

### Overview

The `pallet-pwroko` pallet introduces a wrapped token, `pwRoko`, which represents native `ROKO` tokens locked within the Substrate runtime. This allows the value of `ROKO` to be used within the EVM environment in a way that conforms to the ERC20 token standard.

### Locking ROKO (Substrate)

To obtain `pwRoko`, users must lock their native `ROKO` tokens.

*   **Extrinsic:** `pwroko.lock(amount)`
*   **Action:** Locks the specified `amount` of native `ROKO` from the caller's account and mints an equivalent amount of `pwRoko` tokens to the same account.
*   **Requirements:** The caller must have sufficient free (non-reserved) `ROKO` balance.
*   **Events:** `pwroko.Locked`, `pwroko.Minted`, `balances.Reserved`.

### Unlocking ROKO (Substrate)

To redeem native `ROKO` for `pwRoko`, users must unlock their tokens.

*   **Extrinsic:** `pwroko.unlock(amount)`
*   **Action:** Burns the specified `amount` of `pwRoko` from the caller's account and unreserves an equivalent amount of the previously locked native `ROKO`, making it free again.
*   **Requirements:** The caller must have sufficient `pwRoko` balance.
*   **Events:** `pwroko.Unlocked`, `pwroko.Burned`, `balances.Unreserved`.

### Using pwRoko within EVM (ERC20 Interface)

Once `ROKO` is locked, the resulting `pwRoko` balance is accessible within the EVM through a precompiled contract acting as an ERC20 token.

*   **Precompile Address:** `0x0000000000000000000000000000000000000500` (Address may vary, confirm in runtime configuration)
*   **Standard:** ERC20
*   **Available Functions (via EVM call to the precompile address):**
    *   `name()` (Optional, if implemented in precompile)
    *   `symbol()` (Optional, if implemented in precompile)
    *   `decimals()` (Optional, if implemented in precompile - likely matches native ROKO decimals)
    *   `totalSupply()`: Returns the total amount of `pwRoko` in circulation.
    *   `balanceOf(address owner)`: Returns the `pwRoko` balance of the specified `owner` address.
    *   `allowance(address owner, address spender)`: Returns the amount the `spender` is allowed to withdraw from the `owner`.
    *   `transfer(address to, uint256 amount)`: Transfers `amount` of `pwRoko` from the caller (`msg.sender`) to the `to` address. Requires sufficient balance.
    *   `approve(address spender, uint256 amount)`: Allows the `spender` to withdraw up to `amount` of `pwRoko` from the caller (`msg.sender`).
    *   `transferFrom(address from, address to, uint256 amount)`: Transfers `amount` of `pwRoko` from the `from` address to the `to` address, using the allowance granted to the caller (`msg.sender`). Requires sufficient balance in the `from` account and sufficient allowance for the caller.

**Note:** Locking (`lock`) and unlocking (`unlock`) operations **must** be performed via Substrate extrinsics. They are not directly exposed through the EVM precompile interface. 