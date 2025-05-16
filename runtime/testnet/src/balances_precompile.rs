use crate::sp_core::H160;
use core::marker::PhantomData;
use polkadot_sdk::sp_core::U256;
use polkadot_sdk::frame_system::{self as frame_system, Config as SysConfig};
use polkadot_sdk::sp_runtime::traits::{Saturating, StaticLookup};
use polkadot_sdk::sp_runtime::MultiAddress;
use pallet_evm::{
    AddressMapping, ExitError, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileResult,
};

/// Function selectors for the balances precompile
pub mod balances_selectors {
    pub const TRANSFER_KEEP_ALIVE: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb]; // Même signature que ERC20 transfer(address,uint256)
}

/// Precompile for balances.transferKeepAlive
pub struct BalancesPrecompile<T>(PhantomData<T>);

impl<T> Precompile for BalancesPrecompile<T>
where
    T: pallet_balances::Config + pallet_evm::Config + SysConfig,
    T::AccountId: From<H160> + Into<H160>,
    <T as pallet_balances::Config>::Balance: TryFrom<U256> + Into<U256> + Saturating + Copy,
{
    fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input().to_vec();

        if input.len() < 4 {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Input too short".into()),
            });
        }

        let selector = &input[0..4];

        match selector {
            s if s == balances_selectors::TRANSFER_KEEP_ALIVE => Self::transfer_keep_alive(handle, &input),
            _ => Err(PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "Unknown selector".into(),
            }),
        }
    }
}

impl<T> BalancesPrecompile<T>
where
    T: pallet_balances::Config + pallet_evm::Config + SysConfig,
    T::AccountId: From<H160> + Into<H160>,
    <T as pallet_balances::Config>::Balance: TryFrom<U256> + Into<U256> + Saturating + Copy,
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
    fn u256_to_balance(value: U256) -> Result<<T as pallet_balances::Config>::Balance, PrecompileFailure> {
        <T as pallet_balances::Config>::Balance::try_from(value).map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other("Amount exceeds maximum supported balance".into()),
        })
    }

    fn transfer_keep_alive(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        let to_h160 = Self::read_address(input, 4)?;
        let amount_u256 = Self::read_uint256(input, 36)?;
        let amount_balance = Self::u256_to_balance(amount_u256)?;

        let from = T::AddressMapping::into_account_id(context.caller);
        let to = T::AddressMapping::into_account_id(to_h160);

        // Execute the transferKeepAlive extrinsic 
        match pallet_balances::Pallet::<T>::transfer_keep_alive(
            frame_system::RawOrigin::Signed(from).into(),
            T::Lookup::unlookup(to),
            amount_balance
        ) {
            Ok(_) => {
                let mut output = [0u8; 32];
                output[31] = 1; // Solidity true
                Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.to_vec() })
            }
            Err(_) => {
                Err(PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output: "Failed to execute transferKeepAlive".into(),
                })
            }
        }
    }
} 