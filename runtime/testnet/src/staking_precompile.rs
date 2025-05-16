use crate::sp_core::H160;
use core::marker::PhantomData;
use polkadot_sdk::sp_core::U256;
use polkadot_sdk::sp_runtime::traits::{Saturating, StaticLookup};
use polkadot_sdk::pallet_staking::{Config, Pallet, RewardDestination};
use polkadot_sdk::frame_system::{Config as SysConfig, RawOrigin};
use pallet_evm::{
    Precompile, PrecompileHandle, PrecompileResult, PrecompileFailure, PrecompileOutput,
    ExitError, ExitRevert, ExitSucceed, AddressMapping
};
use polkadot_sdk::sp_std::borrow::Cow;
use polkadot_sdk::sp_std::vec::Vec;

/// Represents the staking selectors
#[allow(dead_code)]
pub mod staking_selectors {
    pub const BOND: [u8; 4] = [0x00, 0x00, 0x00, 0x01]; // bond(uint256, uint8)
    pub const UNBOND: [u8; 4] = [0x00, 0x00, 0x00, 0x02]; // unbond(uint256)
    pub const NOMINATE: [u8; 4] = [0x00, 0x00, 0x00, 0x03]; // nominate(address[])
    pub const WITHDRAW_UNBONDED: [u8; 4] = [0x00, 0x00, 0x00, 0x04]; // withdraw_unbonded(uint32)
}

/// Precompile for staking.bond
pub struct StakingPrecompile<T>(PhantomData<T>);

impl<T> Precompile for StakingPrecompile<T>
where
    T: Config + pallet_evm::Config + SysConfig,
    T::AccountId: From<H160> + Into<H160>,
    <T as Config>::CurrencyBalance: TryFrom<U256> + Into<U256> + Saturating + Copy,
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
            s if s == staking_selectors::BOND => Self::bond(handle, &input),
            s if s == staking_selectors::UNBOND => Self::unbond(handle, &input),
            s if s == staking_selectors::NOMINATE => Self::nominate(handle, &input),
            s if s == staking_selectors::WITHDRAW_UNBONDED => Self::withdraw_unbonded(handle, &input),
            _ => Err(PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "Unknown selector".into(),
            }),
        }
    }
}

impl<T> StakingPrecompile<T>
where
    T: Config + pallet_evm::Config + SysConfig,
    T::AccountId: From<H160> + Into<H160>,
    <T as Config>::CurrencyBalance: TryFrom<U256> + Into<U256> + Saturating + Copy,
{
    // Helper to parse uint256 from input
    fn read_uint256(input: &[u8], offset: usize) -> Result<U256, PrecompileFailure> {
        if input.len() < offset + 32 {
            return Err(PrecompileFailure::Error { 
                exit_status: ExitError::Other("uint256 parse error: input too short".into()) 
            });
        }
        Ok(U256::from_big_endian(&input[offset..offset+32]))
    }

    // Helper to read an address (H160) from input
    fn read_address(input: &[u8], offset: usize) -> Result<H160, PrecompileFailure> {
        if input.len() < offset + 20 {
            return Err(PrecompileFailure::Error { 
                exit_status: ExitError::Other("address parse error: input too short".into()) 
            });
        }
        let mut address = [0u8; 20];
        address.copy_from_slice(&input[offset..offset+20]);
        Ok(H160::from(address))
    }

    // Helper to read a uint32 from input
    fn read_uint32(input: &[u8], offset: usize) -> Result<u32, PrecompileFailure> {
        if input.len() < offset + 4 {
            return Err(PrecompileFailure::Error { 
                exit_status: ExitError::Other("uint32 parse error: input too short".into()) 
            });
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&input[offset..offset+4]);
        Ok(u32::from_be_bytes(bytes))
    }

    // Helper to convert U256 amount to pallet's Balance
    fn u256_to_balance(value: U256) -> Result<<T as Config>::CurrencyBalance, PrecompileFailure> {
        <T as Config>::CurrencyBalance::try_from(value).map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other("Amount exceeds maximum supported balance".into()),
        })
    }

    /// Convertit un u8 en RewardDestination
    fn u8_to_reward_destination(payee_type: u8) -> Result<RewardDestination<T::AccountId>, PrecompileFailure> {
        match payee_type {
            0 => Ok(RewardDestination::Staked),
            1 => Ok(RewardDestination::Stash),
            2 => Ok(RewardDestination::Controller),
            _ => Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Invalid payee type. Valid values: 0=Staked, 1=Stash, 2=Controller".into()),
            }),
        }
    }

    // Prépare le succès output (true) sous forme de bytes
    fn success_output() -> PrecompileOutput {
        let mut output = [0u8; 32];
        output[31] = 1;
        PrecompileOutput { 
            exit_status: ExitSucceed::Returned, 
            output: output.to_vec() 
        }
    }

    // Convertit n'importe quelle erreur en PrecompileFailure
    fn handle_error<E: core::fmt::Debug>(e: E) -> PrecompileFailure {
        log::info!("DEBUG: error = {:?}", e);
        PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("Execution failed"))
        }
    }

    fn bond(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        
        // Lire le montant
        let amount_u256 = if input.len() >= 36 {
            Self::read_uint256(input, 4)?
        } else {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Amount not provided".into()),
            });
        };
        
        let amount = Self::u256_to_balance(amount_u256)?;
        
        // Lire le payee_type
        let payee_type = if input.len() >= 37 {
            input[36]
        } else {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Payee type not provided".into()),
            });
        };
        
        log::info!("DEBUG: payee_type = {}", payee_type);
        
        // Convertir en RewardDestination
        let reward_destination = Self::u8_to_reward_destination(payee_type)?;
        
        let account_id = T::AddressMapping::into_account_id(context.caller);
        let origin = RawOrigin::Signed(account_id).into();
        
        // Appeler la fonction du pallet staking
        match Pallet::<T>::bond(origin, amount, reward_destination) {
            Ok(_) => Ok(Self::success_output()),
            Err(e) => Err(Self::handle_error(e))
        }
    }

    fn unbond(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        
        // Lire le montant
        let amount_u256 = if input.len() >= 36 {
            Self::read_uint256(input, 4)?
        } else {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Amount not provided".into()),
            });
        };
        
        let amount = Self::u256_to_balance(amount_u256)?;
        let account_id = T::AddressMapping::into_account_id(context.caller);
        let origin = RawOrigin::Signed(account_id).into();
        
        // Appeler la fonction du pallet staking
        match Pallet::<T>::unbond(origin, amount) {
            Ok(_) => Ok(Self::success_output()),
            Err(e) => Err(Self::handle_error(e))
        }
    }

    fn nominate(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        
        // Lire le nombre de validateurs
        if input.len() < 36 {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Input too short".into()),
            });
        }
        
        // Lire le nombre de validateurs depuis les 32 premiers octets après le sélecteur
        let count_u256 = Self::read_uint256(input, 4)?;
        let count = count_u256.as_u64() as usize;
        
        if count == 0 {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("No validators provided".into()),
            });
        }
        
        // Vérifier que l'input est assez long pour contenir tous les validateurs
        if input.len() < 36 + (count * 32) {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("Input too short for validator count".into()),
            });
        }
        
        // Lire les adresses des validateurs (format Ethereum)
        let mut targets = Vec::with_capacity(count);
        for i in 0..count {
            // Dans l'ABI Ethereum, les adresses sont paddées à 32 bytes
            let offset = 36 + (i * 32) + 12; // 4 (selector) + 32 (count) + i*32 (address offset) + 12 (padding)
            let address = Self::read_address(input, offset)?;
            let account_id = T::AddressMapping::into_account_id(address);
            // Conversion en StaticLookup::Source
            targets.push(T::Lookup::unlookup(account_id));
        }
        
        let account_id = T::AddressMapping::into_account_id(context.caller);
        let origin = RawOrigin::Signed(account_id).into();
        
        // Appeler la fonction du pallet staking
        match Pallet::<T>::nominate(origin, targets) {
            Ok(_) => Ok(Self::success_output()),
            Err(e) => Err(Self::handle_error(e))
        }
    }

    fn withdraw_unbonded(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let context = handle.context();
        
        // Lire le nombre de slashing spans à soustraire (optionnel)
        let num_slashing_spans = if input.len() >= 36 {
            // Lire les 4 derniers octets du paramètre uint32 (big-endian)
            let value_u256 = Self::read_uint256(input, 4)?;
            value_u256.as_u32()
        } else {
            0 // Valeur par défaut
        };
        
        let account_id = T::AddressMapping::into_account_id(context.caller);
        let origin = RawOrigin::Signed(account_id).into();
        
        // Appeler la fonction du pallet staking
        match Pallet::<T>::withdraw_unbonded(origin, num_slashing_spans) {
            Ok(_) => Ok(Self::success_output()),
            Err(e) => Err(Self::handle_error(e))
        }
    }
} 