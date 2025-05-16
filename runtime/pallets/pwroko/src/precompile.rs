use core::marker::PhantomData;
// Use imports relative to the Substrate ecosystem, not polkadot_sdk directly unless necessary
use sp_core::{H160, U256};
// use frame_system; // Removed - Keep this for frame_system::Config bound
use pallet_evm::{
    // Removed Context, ExitReason
    AddressMapping, ExitError, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileResult,
};
use sp_runtime::{traits::Saturating, DispatchError}; // Add DispatchError, Keep Saturating
use frame_system::RawOrigin;

// --- Custom Precompile for pwRoko ---

/// Represents the standard ERC20 selectors
#[allow(dead_code)]
mod erc20_selectors {
    pub const TOTAL_SUPPLY: [u8; 4] = [0x18, 0x16, 0x0d, 0xdd]; // totalSupply()
    pub const BALANCE_OF: [u8; 4] = [0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
    pub const ALLOWANCE: [u8; 4] = [0xdd, 0x62, 0xed, 0x3e]; // allowance(address,address)
    pub const TRANSFER: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb]; // transfer(address,uint256)
    pub const APPROVE: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3]; // approve(address,uint256)
    pub const TRANSFER_FROM: [u8; 4] = [0x23, 0xb8, 0x72, 0xdd]; // transferFrom(address,address,uint256)
    // Optional
    pub const NAME: [u8; 4] = [0x06, 0xfd, 0xde, 0x03]; // name()
    pub const SYMBOL: [u8; 4] = [0x95, 0xd8, 0x9b, 0x41]; // symbol()
    pub const DECIMALS: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67]; // decimals()
    // Lock and Unlock selectors
    pub const LOCK: [u8; 4] = [0xdd, 0x46, 0x70, 0x64]; // lock(uint256)
    pub const UNLOCK: [u8; 4] = [0x2f, 0x87, 0x88, 0xc4]; // unlock(uint256)
}

// Need to import pallet's Config, Pallet, BalanceOf etc.
use crate::{BalanceOf, Config, Pallet}; // Import Error enum for mapping

/// Wrapper struct for the pwRoko precompile
pub struct PwRokoPrecompile<T>(PhantomData<T>);

impl<T> Precompile for PwRokoPrecompile<T>
where
    // Use the pallet's Config trait, require EVM config from runtime
    T: Config + pallet_evm::Config,
    // Ensure AccountId can be converted from/to H160 via AddressMapping from EVM config
    T::AccountId: From<H160> + Into<H160>,
    // Ensure pallet's Balance can be converted from/to U256 and supports Saturating
    BalanceOf<T>: TryFrom<U256> + Into<U256> + Saturating + Copy, // Add Copy bound if needed by pallet functions
{
    fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input().to_vec();

        if input.len() < 4 {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Input too short".into()),
            });
        }

        // Consider base cost? Pallet weights should cover execution.
        // handle.record_cost(Pallet::<T>::weight_info().some_base_cost())?; // Example

        let selector = &input[0..4];

        match selector {
            s if s == erc20_selectors::TOTAL_SUPPLY => Self::total_supply(handle),
            s if s == erc20_selectors::BALANCE_OF => Self::balance_of(handle, &input),
            s if s == erc20_selectors::ALLOWANCE => Self::allowance(handle, &input),
            s if s == erc20_selectors::TRANSFER => Self::transfer(handle, &input),
            s if s == erc20_selectors::APPROVE => Self::approve(handle, &input),
            s if s == erc20_selectors::TRANSFER_FROM => Self::transfer_from(handle, &input),
            s if s == erc20_selectors::LOCK => Self::lock(handle, &input),
            s if s == erc20_selectors::UNLOCK => Self::unlock(handle, &input),
            _ => Err(PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted, // Use Revert for unknown selector
                output: "Unknown selector".into(),
            }),
        }
    }
}

impl<T> PwRokoPrecompile<T>
where
    // Repeat bounds from Precompile impl
    T: Config + pallet_evm::Config,
    T::AccountId: From<H160> + Into<H160>,
    BalanceOf<T>: TryFrom<U256> + Into<U256> + Saturating + Copy,
{
    // Helper to parse ERC20 address from input
    fn read_address(input: &[u8], offset: usize) -> Result<H160, PrecompileFailure> {
        if input.len() < offset + 32 {
            return Err(PrecompileFailure::Error { exit_status: ExitError::Other("Address parse error: input too short".into()) });
        }
        Ok(H160::from_slice(&input[offset+12..offset+32]))
    }

    // Helper to parse ERC20 uint256 from input
    fn read_uint256(input: &[u8], offset: usize) -> Result<U256, PrecompileFailure> {
        if input.len() < offset + 32 {
            return Err(PrecompileFailure::Error { exit_status: ExitError::Other("uint256 parse error: input too short".into()) });
        }
        Ok(U256::from_big_endian(&input[offset..offset+32]))
    }

    // Helper to convert U256 amount to pallet's Balance, checking for truncation
    fn u256_to_balance(value: U256) -> Result<BalanceOf<T>, PrecompileFailure> {
        BalanceOf::<T>::try_from(value).map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other("Amount exceeds maximum supported balance".into()),
        })
    }

    // Map pallet DispatchError to PrecompileFailure
    fn map_dispatch_error(err: DispatchError) -> PrecompileFailure {
        let error_message: &'static str = err.into(); // Convert DispatchError to static str
        PrecompileFailure::Error {
            // Use Revert to provide a reason string?
            // exit_status: ExitRevert::Reverted,
            // output: error_message.into(),
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(error_message))
        }
    }


    fn total_supply(_handle: &mut impl PrecompileHandle) -> PrecompileResult { // handle -> _handle
        let total_supply_pallet = Pallet::<T>::total_supply();
        let total_supply_u256: U256 = total_supply_pallet.into();

        let mut output = [0u8; 32];
        total_supply_u256.to_big_endian(&mut output);

        // TODO: Record cost based on weight
        // _handle.record_cost(Pallet::<T>::weight_info().total_supply())?;

        Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
    }

    fn balance_of(_handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult { // handle -> _handle
        let owner_h160 = Self::read_address(input, 4)?;
        let owner_account_id = T::AddressMapping::into_account_id(owner_h160);

        let balance_pallet = Pallet::<T>::balances(&owner_account_id);
        let balance_u256: U256 = balance_pallet.into();

        let mut output = [0u8; 32];
        balance_u256.to_big_endian(&mut output);

        // TODO: Record cost
        // _handle.record_cost(Pallet::<T>::weight_info().balance_of())?;

        Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
    }

    fn allowance(_handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult { // handle -> _handle
        let owner_h160 = Self::read_address(input, 4)?;
        let spender_h160 = Self::read_address(input, 36)?;

        let owner_account_id = T::AddressMapping::into_account_id(owner_h160);
        let spender_account_id = T::AddressMapping::into_account_id(spender_h160);

        let allowance_pallet = Pallet::<T>::allowances(&owner_account_id, &spender_account_id);
        let allowance_u256: U256 = allowance_pallet.into();

        let mut output = [0u8; 32];
        allowance_u256.to_big_endian(&mut output);

        // TODO: Record cost
        // _handle.record_cost(Pallet::<T>::weight_info().allowance())?;

        Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
    }

    fn transfer(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        let to_h160 = Self::read_address(input, 4)?;
        let amount_u256 = Self::read_uint256(input, 36)?;
        let amount_pallet = Self::u256_to_balance(amount_u256)?;

        let origin = T::AddressMapping::into_account_id(context.caller);
        let to = T::AddressMapping::into_account_id(to_h160);

        // TODO: Record cost BEFORE execution based on weight
        // handle.record_cost(Pallet::<T>::weight_info().transfer())?;

        match Pallet::<T>::do_transfer(origin, to, amount_pallet) {
            Ok(_) => {
                let mut output = [0u8; 32];
                output[31] = 1; // Solidity true
                Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
            }
            Err(e) => Err(Self::map_dispatch_error(e))
        }
    }

    fn approve(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        let spender_h160 = Self::read_address(input, 4)?;
        let amount_u256 = Self::read_uint256(input, 36)?;
        let amount_pallet = Self::u256_to_balance(amount_u256)?;

        let owner = T::AddressMapping::into_account_id(context.caller);
        let spender = T::AddressMapping::into_account_id(spender_h160);

        // TODO: Record cost
        // handle.record_cost(Pallet::<T>::weight_info().approve())?;

        // Use pallet's internal approve_spending helper
        Pallet::<T>::approve_spending(owner, spender, amount_pallet);

        let mut output = [0u8; 32];
        output[31] = 1; // Solidity true
        Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
    }

    fn transfer_from(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        let from_h160 = Self::read_address(input, 4)?;
        let to_h160 = Self::read_address(input, 36)?;
        let amount_u256 = Self::read_uint256(input, 68)?;
        let amount_pallet = Self::u256_to_balance(amount_u256)?;

        let spender_account_id = T::AddressMapping::into_account_id(context.caller);
        let from_account_id = T::AddressMapping::into_account_id(from_h160);
        let to_account_id = T::AddressMapping::into_account_id(to_h160);

        // TODO: Record cost
        // handle.record_cost(Pallet::<T>::weight_info().transfer_from())?;

        // Check allowance first
        let current_allowance = Pallet::<T>::allowances(&from_account_id, &spender_account_id);
        if current_allowance < amount_pallet {
            // Consider using Revert for specific ERC20 errors like insufficient allowance
             return Err(PrecompileFailure::Revert {
                 exit_status: ExitRevert::Reverted,
                 output: "ERC20: insufficient allowance".into(),
             });
        }

        // Attempt transfer
        match Pallet::<T>::do_transfer(from_account_id.clone(), to_account_id, amount_pallet) {
            Ok(_) => {
                // Decrease allowance
                let new_allowance = current_allowance.saturating_sub(amount_pallet);
                Pallet::<T>::approve_spending(from_account_id, spender_account_id, new_allowance);

                let mut output = [0u8; 32];
                output[31] = 1; // Solidity true
                Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
            }
            Err(e) => Err(Self::map_dispatch_error(e))
        }
    }

    /// Lock native tokens to mint pwRoko tokens
    fn lock(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        let amount_u256 = Self::read_uint256(input, 4)?;
        let amount_pallet = Self::u256_to_balance(amount_u256)?;

        let account_id = T::AddressMapping::into_account_id(context.caller);
        let origin = RawOrigin::Signed(account_id).into();

        // TODO: Record cost based on weight
        // handle.record_cost(Pallet::<T>::weight_info().lock())?;

        match Pallet::<T>::lock(origin, amount_pallet) {
            Ok(_) => {
                let mut output = [0u8; 32];
                output[31] = 1; // Solidity true
                Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
            }
            Err(e) => Err(Self::map_dispatch_error(e))
        }
    }

    /// Unlock native tokens by burning pwRoko tokens
    fn unlock(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        let amount_u256 = Self::read_uint256(input, 4)?;
        let amount_pallet = Self::u256_to_balance(amount_u256)?;

        let account_id = T::AddressMapping::into_account_id(context.caller);
        let origin = RawOrigin::Signed(account_id).into();

        // TODO: Record cost based on weight
        // handle.record_cost(Pallet::<T>::weight_info().unlock())?;

        match Pallet::<T>::unlock(origin, amount_pallet) {
            Ok(_) => {
                let mut output = [0u8; 32];
                output[31] = 1; // Solidity true
                Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
            }
            Err(e) => Err(Self::map_dispatch_error(e))
        }
    }
} 