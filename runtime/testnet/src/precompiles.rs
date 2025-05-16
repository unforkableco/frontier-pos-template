use crate::sp_core::H160;
use core::marker::PhantomData;
use polkadot_sdk::sp_core::U256;
use polkadot_sdk::frame_system::Config as SysConfig;
use pallet_evm::{
    IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult, PrecompileSet,
    PrecompileFailure, PrecompileOutput, ExitError, ExitRevert, ExitSucceed, AddressMapping
};

use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};

// Import the PwRoko precompile from its pallet
use pallet_pwroko::PwRokoPrecompile;
// Import the Staking precompile
use crate::staking_precompile::StakingPrecompile;
// Import the Balances precompile
use crate::balances_precompile::BalancesPrecompile;
// Import pallet_staking
use polkadot_sdk::pallet_staking;

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self(Default::default())
    }
    pub fn used_addresses() -> [H160; 10] {
        [
            hash(1),
            hash(2),
            hash(3),
            hash(4),
            hash(5),
            hash(1024),
            hash(1025),
            // Custom precompile address for pwRoko
            hash(0x500), // Address 0x0...0500
            // Custom precompile address for balances.transferKeepAlive
            hash(0x600), // Address 0x0...0600
            // Custom precompile address for staking.bond
            hash(0x700), // Address 0x0...0700
        ]
    }
}
impl<R> PrecompileSet for FrontierPrecompiles<R>
where
    R: pallet_evm::Config + pallet_pwroko::Config + pallet_balances::Config + pallet_staking::Config + SysConfig,
    R::AccountId: From<H160> + Into<H160>,
    pallet_pwroko::BalanceOf<R>: TryFrom<U256> + Into<U256> + Copy,
    <R as pallet_balances::Config>::Balance: TryFrom<U256> + Into<U256> + Copy,
    <R as pallet_staking::Config>::CurrencyBalance: TryFrom<U256> + Into<U256> + Copy,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Ethereum precompiles :
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            // Non-Frontier specific nor Ethereum precompiles :
            a if a == hash(1024) => Some(Sha3FIPS256::execute(handle)),
            a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
            // Custom precompiles:
            a if a == hash(0x500) => Some(PwRokoPrecompile::<R>::execute(handle)),
            a if a == hash(0x600) => Some(BalancesPrecompile::<R>::execute(handle)),
            a if a == hash(0x700) => Some(StakingPrecompile::<R>::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
