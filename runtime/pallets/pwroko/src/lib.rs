//! Pallet for managing a wrapped ROKO token (pwROKO).

#![cfg_attr(not(feature = "std"), no_std)]

// Use ensure macro
use frame_support::ensure;
use frame_support::traits::{LockableCurrency as LockableCurrencyTrait, WithdrawReasons as FrameWithdrawReasons};
use sp_runtime::DispatchResult;
use frame_system::pallet_prelude::BlockNumberFor;

// Define constants outside mod pallet so external impls can access them
const TOKEN_SYMBOL: &'static [u8] = b"pwROKO";
const DECIMALS: u8 = 18;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

// Add precompile module and export the precompile struct
// Only include this module if the pallet-evm feature is enabled
#[cfg(feature = "pallet-evm")]
pub mod precompile;
#[cfg(feature = "pallet-evm")]
pub use precompile::PwRokoPrecompile;

// --- Consolidated Imports for Traits --- 
use frame_support::traits::{ 
    tokens::{Precision, Preservation, Fortitude, WithdrawConsequence, DepositConsequence, Provenance, fungible::{self, Inspect, Mutate}},
    WithdrawReasons, LockIdentifier
}; 
use sp_runtime::traits::{Zero, Saturating, CheckedAdd, CheckedSub, Bounded};
use frame_support::traits::tokens::ExistenceRequirement;
use frame_support::traits::Imbalance;
use sp_runtime::{DispatchError, ArithmeticError, TokenError};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;
    use frame_support::pallet_prelude::*;
    use frame_support::{dispatch::DispatchResult, traits::{ReservableCurrency, WithdrawReasons as FrameWithdrawReasons, Currency as FrameCurrency}};
    use sp_runtime::traits::{Zero, Saturating, CheckedAdd, CheckedSub};
    use sp_std::vec::Vec;

    // TODO: Potentially import WeightInfo if benchmarks are added
    // use crate::WeightInfo;

    // DEFINE BalanceOf HERE
    pub type BalanceOf<T> = <<T as Config>::Currency as frame_support::traits::Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency type for managing ROKO (native token).
        type Currency: ReservableCurrency<Self::AccountId>;

        // Add WeightInfo since weights are required
        type WeightInfo: WeightInfo;

        /// The maximum number of locks that can be placed on an account.
        type MaxLocks: Get<u32>;

        // TODO: Consider adding constants like MaxLocks or similar if needed.
    }

    // --- Storage --- 

    /// Stores the amount of native ROKO locked by each account.
    #[pallet::storage]
    #[pallet::getter(fn locked_balances)]
    pub type LockedBalances<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Stores the balance of the wrapped pwRoko token for each account.
    #[pallet::storage]
    #[pallet::getter(fn balances)]
    pub type Balances<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Stores the total supply of the wrapped pwRoko token.
    #[pallet::storage]
    #[pallet::getter(fn total_supply)]
    pub type TotalSupply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Stores the ERC20-like allowances.
    #[pallet::storage]
    #[pallet::getter(fn allowances)]
    pub type Allowances<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, T::AccountId, // owner
        Blake2_128Concat, T::AccountId, // spender
        BalanceOf<T>, // allowance
        ValueQuery,
    >;

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub enum PwRokoWithdrawReasons {
        Fee,
        Transfer,
        Reserve,
        Other,
    }

    impl From<FrameWithdrawReasons> for PwRokoWithdrawReasons {
        fn from(reasons: FrameWithdrawReasons) -> Self {
            if reasons == FrameWithdrawReasons::TRANSACTION_PAYMENT {
                Self::Fee
            } else if reasons == FrameWithdrawReasons::TRANSFER {
                Self::Transfer
            } else if reasons == FrameWithdrawReasons::RESERVE {
                Self::Reserve
            } else {
                Self::Other // A general category for other bitflags
            }
        }
    }

    /// Defines the balance lock structure.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub struct BalanceLock<Balance> {
        pub id: LockIdentifier,
        pub amount: Balance,
        pub reasons: PwRokoWithdrawReasons,
    }

    /// Stores the locks placed on accounts.
    #[pallet::storage]
    #[pallet::getter(fn locks)]
    pub type Locks<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<BalanceLock<BalanceOf<T>>, T::MaxLocks>, ValueQuery>;

    // --- Events --- 

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Native currency locked.
        Locked { who: T::AccountId, amount: BalanceOf<T> },
        /// Native currency unlocked.
        Unlocked { who: T::AccountId, amount: BalanceOf<T> },
        /// Wrapped token minted.
        Minted { to: T::AccountId, amount: BalanceOf<T> },
        /// Wrapped token burned.
        Burned { from: T::AccountId, amount: BalanceOf<T> },
        /// Wrapped token transferred.
        Transfer { from: T::AccountId, to: T::AccountId, amount: BalanceOf<T> },
        /// Wrapped token allowance approved.
        Approval { owner: T::AccountId, spender: T::AccountId, amount: BalanceOf<T> },
    }

    // --- Errors --- 

    #[pallet::error]
    pub enum Error<T> {
        /// The amount specified is zero.
        AmountZero,
        /// The account has insufficient native balance to lock.
        InsufficientNativeBalance,
        /// The account has insufficient locked native balance to perform the unlock.
        InsufficientLockedBalance,
        /// The account has insufficient wrapped token balance.
        InsufficientWrappedBalance,
        /// An arithmetic overflow occurred.
        Overflow,
        /// The account has insufficient allowance to transfer funds.
        InsufficientAllowance,
        /// The operation would result in dust balance cleanup, which is not supported for wrapped token.
        /// Or, the sender tries to send funds to self.
        /// Or, potentially other invalid transfer scenarios.
        InvalidTransfer,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// Initial balances for pwRoko.
        /// (AccountId, Balance)
        pub balances: Vec<(T::AccountId, BalanceOf<T>)>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { balances: Vec::new() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let mut total_supply: BalanceOf<T> = Zero::zero();
            for (who, amount) in &self.balances {
                // Using fungible::Inspect::minimum_balance() to check.
                // Ensure Pallet<T> implements fungible::Inspect<T::AccountId>.
                // This check might need adjustment based on exact trait bounds and pallet structure.
                // For now, assuming direct access or a helper.
                // A common pattern is Pallet::<T>::minimum_balance() if Inspect is implemented.
                // ensure!(*amount >= Pallet::<T>::minimum_balance(), "Initial balance below minimum");
                // A simpler assertion for now if minimum_balance is always zero or not strictly enforced at genesis for this token.
                // If pwROKO can have zero as min_balance, this assert is fine.
                // Otherwise, it needs to be compared with the actual minimum_balance.
                // For pwROKO, it's likely Zero.
                assert!(*amount >= <Pallet<T> as fungible::Inspect<T::AccountId>>::minimum_balance(), "Initial balance is below minimum_balance()");


                Balances::<T>::insert(who, amount);
                total_supply = total_supply.saturating_add(*amount);
            }
            TotalSupply::<T>::put(total_supply);
        }
    }

    // --- Extrinsics --- 

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Lock native ROKO tokens to mint wrapped pwRoko tokens.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::lock())]
        pub fn lock(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

            // Reserve native currency
            T::Currency::reserve(&who, amount).map_err(|_| Error::<T>::InsufficientNativeBalance)?;

            // Update locked balance storage
            LockedBalances::<T>::try_mutate(&who, |locked_balance| -> DispatchResult {
                *locked_balance = locked_balance.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            // Mint wrapped token (Implementation below)
            Self::do_mint(&who, amount)?;

            Self::deposit_event(Event::Locked { who, amount });
            Ok(())
        }

        /// Unlock native ROKO tokens by burning wrapped pwRoko tokens.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::unlock())]
        pub fn unlock(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

            // Burn wrapped token (Implementation below)
            Self::do_burn(&who, amount)?;

            // Check if enough native currency is actually locked 
            // (This check should technically be covered by the burn logic ensuring pwRoko balance == locked ROKO)
            // but an explicit check against LockedBalances storage adds safety.
            LockedBalances::<T>::try_mutate(&who, |locked_balance| -> DispatchResult {
                *locked_balance = locked_balance.checked_sub(&amount).ok_or(Error::<T>::InsufficientLockedBalance)?;
                Ok(())
            })?;

            // Unreserve native currency
            // Note: unreserve returns the actual amount unreserved.
            // We expect it to be equal to 'amount' if everything is correct.
            let unreserved_amount = T::Currency::unreserve(&who, amount);
            ensure!(unreserved_amount == amount, Error::<T>::InsufficientLockedBalance); // Should not happen if logic is correct

            Self::deposit_event(Event::Unlocked { who, amount });
            Ok(())
        }

        /// Transfer wrapped pwRoko tokens to another account.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(origin: OriginFor<T>, to: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            Self::do_transfer(from, to, amount)
        }

        /// Approve a spender to withdraw wrapped pwRoko tokens from the caller's account.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::approve())]
        pub fn approve(origin: OriginFor<T>, spender: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let owner = ensure_signed(origin)?;

            // Use the internal helper which also emits the event
            Self::approve_spending(owner, spender, amount);
            // Allowances::<T>::insert(&owner, &spender, amount); // Logic moved to helper
            // Self::deposit_event(Event::Approval { owner, spender, amount }); // Logic moved to helper
            Ok(())
        }

        /// Transfer wrapped pwRoko tokens from one account to another, using allowance.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::transfer_from())]
        pub fn transfer_from(origin: OriginFor<T>, from: T::AccountId, to: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let spender = ensure_signed(origin)?;
            ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

            let current_allowance = Allowances::<T>::get(&from, &spender);
            ensure!(current_allowance >= amount, Error::<T>::InsufficientAllowance);

            // Perform the transfer
            Self::do_transfer(from.clone(), to, amount)?;

            // Decrease allowance
            let new_allowance = current_allowance.saturating_sub(amount); // Use saturating_sub for safety
            Allowances::<T>::insert(&from, &spender, new_allowance);
            // Note: No Approval event is emitted on transfer_from according to ERC20.

            Ok(())
        }
    }

    // --- Internal Helper Functions ---

    impl<T: Config> Pallet<T> {
        /// Internal function to handle minting of pwRoko.
        pub fn do_mint(to: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

            Balances::<T>::try_mutate(to, |balance| -> DispatchResult {
                *balance = balance.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            TotalSupply::<T>::try_mutate(|total_supply| -> DispatchResult {
                *total_supply = total_supply.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            Self::deposit_event(Event::Minted { to: to.clone(), amount });
            Ok(())
        }

        /// Internal function to handle burning of pwRoko.
        pub fn do_burn(from: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

            Balances::<T>::try_mutate(from, |balance| -> DispatchResult {
                *balance = balance.checked_sub(&amount).ok_or(Error::<T>::InsufficientWrappedBalance)?;
                Ok(())
            })?;

            TotalSupply::<T>::try_mutate(|total_supply| -> DispatchResult {
                // Total supply should always be greater than or equal to amount if balances are consistent
                *total_supply = total_supply.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            Self::deposit_event(Event::Burned { from: from.clone(), amount });
            Ok(())
        }

        /// Internal function to handle transferring pwRoko.
        pub fn do_transfer(from: T::AccountId, to: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            ensure!(from != to, Error::<T>::InvalidTransfer);
            ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

            Balances::<T>::try_mutate(&from, |from_balance| -> DispatchResult {
                *from_balance = from_balance.checked_sub(&amount).ok_or(Error::<T>::InsufficientWrappedBalance)?;
                Ok(())
            })?;

            Balances::<T>::try_mutate(&to, |to_balance| -> DispatchResult {
                *to_balance = to_balance.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            Self::deposit_event(Event::Transfer { from, to, amount });
            Ok(())
        }

        // Move approve_spending here to keep helpers together
        /// Internal or helper function to directly set/update allowance.
        /// This allows the precompile to manage allowances without needing a signed origin.
        /// Also used by the `approve` extrinsic.
        pub fn approve_spending(owner: T::AccountId, spender: T::AccountId, amount: BalanceOf<T>) {
            Allowances::<T>::insert(&owner, &spender, amount);
            // Emit the Approval event
            Self::deposit_event(Event::Approval { owner, spender, amount });
        }

        // --- Public getters matching ERC20 interface (callable from precompiles/other pallets) ---
        // Note: Storage getters are already generated by #[pallet::getter(fn ...)]

        // We might add explicit functions here if more complex logic beyond direct storage access is needed,
        // or to provide a more stable internal API than direct storage access.
        // For example:
        // pub fn balance_of(who: &T::AccountId) -> BalanceOf<T> {
        //     Self::balances(who)
        // }
        // pub fn total_supply_pwroko() -> BalanceOf<T> {
        //     Self::total_supply()
        // }
        // pub fn allowance_of(owner: &T::AccountId, spender: &T::AccountId) -> BalanceOf<T> {
        //     Self::allowances(owner, spender)
        // }
    }

} 

// --- Trait Implementations --- 

// Imports needed specifically for the fungible traits implementation - REMOVED DUPLICATE BLOCK

// --- Inspect Trait Implementation ---
impl<T: Config> fungible::Inspect<T::AccountId> for Pallet<T> {
    type Balance = BalanceOf<T>;

    // Removed token_symbol() and decimals()

    fn total_issuance() -> Self::Balance {
        TotalSupply::<T>::get()
    }

    fn minimum_balance() -> Self::Balance {
        Zero::zero()
    }

    fn balance(who: &T::AccountId) -> Self::Balance {
        Balances::<T>::get(who)
    }

    // Added missing total_balance function
    fn total_balance(who: &T::AccountId) -> Self::Balance {
        // For pwRoko, currently total_balance is the same as free balance
        // as this pallet doesn't manage internal reserves/freezes.
        Self::balance(who)
    }

    fn reducible_balance(who: &T::AccountId, preservation: Preservation, _force: Fortitude) -> Self::Balance {
        let bal = Self::balance(who);
        // Ensure ED is respected based on preservation strategy
        match preservation {
            Preservation::Protect if bal == Self::minimum_balance() => Zero::zero(),
            _ => bal,
        }
    }

    // Corrected can_deposit (ensure no ArithmeticError::BelowMinimum as it doesn't exist)
    fn can_deposit(who: &T::AccountId, amount: Self::Balance, _provenance: Provenance) -> DepositConsequence {
        if amount.is_zero() {
            return DepositConsequence::Success;
        }
        let _new_balance = match Self::balance(who).checked_add(&amount) {
            Some(nb) => nb,
            None => return DepositConsequence::Overflow,
        };
        DepositConsequence::Success
    }
    
    // Corrected can_withdraw (ensure no ArithmeticError::BelowMinimum/Unknown)
    fn can_withdraw(
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> WithdrawConsequence<Self::Balance> {
         if amount.is_zero() {
            return WithdrawConsequence::Success;
        }
        let current_balance = Self::balance(who);
        if current_balance < amount {
            return WithdrawConsequence::BalanceLow;
        }
        match current_balance.checked_sub(&amount) {
            Some(new_balance) => {
                // With ED=0, ReducedToZero covers the case where new_balance is 0.
                 if new_balance < Self::minimum_balance() { // This condition is equivalent to `new_balance == 0` when ED=0
                    WithdrawConsequence::ReducedToZero(new_balance)
                } else {
                    WithdrawConsequence::Success
                }
            }
            None => { 
                // Underflow despite balance check suggests logic error or non-standard arithmetic
                WithdrawConsequence::Underflow 
            }
        }
    }
}

// --- Mutate Trait Implementation ---
impl<T: Config> fungible::Mutate<T::AccountId> for Pallet<T> {
    fn mint_into(who: &T::AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError> {
        if amount.is_zero() { return Ok(Zero::zero()); }
        let new_total_supply = TotalSupply::<T>::get().checked_add(&amount)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        Self::do_mint(who, amount)?;
        ensure!(TotalSupply::<T>::get() == new_total_supply, DispatchError::Arithmetic(ArithmeticError::Overflow));
        Ok(amount)
    }

    // Corrected burn_from error mapping
    fn burn_from(who: &T::AccountId, amount: Self::Balance, _preservation: Preservation, _precision: Precision, _fortitude: Fortitude) -> Result<Self::Balance, DispatchError> {
        if amount.is_zero() { return Ok(Zero::zero()); }

        let withdraw_result = Self::can_withdraw(who, amount);
        ensure!(
            matches!(withdraw_result, WithdrawConsequence::Success | WithdrawConsequence::ReducedToZero(_)),
            match withdraw_result {
                WithdrawConsequence::BalanceLow => DispatchError::Token(TokenError::FundsUnavailable), 
                WithdrawConsequence::Underflow => DispatchError::Arithmetic(ArithmeticError::Underflow), 
                _ => DispatchError::Other("UnexpectedWithdrawConsequence"),
            }
        );

        Self::do_burn(who, amount)?;
        Ok(amount)
    }
    
    fn set_balance(who: &T::AccountId, amount: Self::Balance) -> Self::Balance { 
        let old_balance = Balances::<T>::get(who);
        if amount == old_balance { return old_balance; }

        // Use saturating_sub for difference calculation to avoid panic on underflow
        let difference = if amount > old_balance { 
            amount.saturating_sub(old_balance)
        } else {
            old_balance.saturating_sub(amount)
        };

        let new_total_supply_result = if amount > old_balance {
            TotalSupply::<T>::get().checked_add(&difference)
        } else {
            TotalSupply::<T>::get().checked_sub(&difference)
        };

        let new_total_supply = new_total_supply_result.unwrap_or_else(|| {
            log::error!("Total supply overflow/underflow in set_balance for {:?}", who);
            if amount > old_balance { Bounded::max_value() } else { Zero::zero() }
        });

        Balances::<T>::insert(who, amount);
        TotalSupply::<T>::put(new_total_supply);
        old_balance
    }
}

// --- InspectLockableCurrency and LockableCurrency Trait Implementation ---
impl<T: Config> LockableCurrencyTrait<T::AccountId> for Pallet<T> {
    type Moment = BlockNumberFor<T>;
    type MaxLocks = T::MaxLocks;

    fn set_lock(
        id: LockIdentifier,
        who: &T::AccountId,
        amount: <Self as fungible::Inspect<T::AccountId>>::Balance,
        reasons: FrameWithdrawReasons
    ) {
        log::warn!(
            "LockableCurrency::set_lock called for {:?} with id {:?}, amount {:?}, reasons {:?}",
            who,
            id,
            amount,
            reasons
        );
        let lock = BalanceLock {
            id,
            amount,
            reasons: reasons.into(),
        };
        let mut locks = Locks::<T>::get(who);
        if let Some(pos) = locks.iter().position(|l| l.id == id) {
            locks[pos] = lock;
        } else {
            if locks.try_push(lock).is_err() {
                log::error!("Too many locks on account {:?}", who);
            }
        }
        Locks::<T>::insert(who, locks);
    }

    fn extend_lock(
        id: LockIdentifier,
        who: &T::AccountId,
        amount: <Self as fungible::Inspect<T::AccountId>>::Balance,
        reasons: FrameWithdrawReasons
    ) {
        log::warn!(
            "LockableCurrency::extend_lock called for {:?} with id {:?}, amount {:?}, reasons {:?}",
            who,
            id,
            amount,
            reasons
        );
        Self::set_lock(id, who, amount, reasons);
    }

    fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
        log::warn!("LockableCurrency::remove_lock called for {:?} with id {:?}", who, id);
        Locks::<T>::mutate(who, |locks| {
            locks.retain(|l| l.id != id);
        });
    }
}

impl<T: Config> frame_support::traits::InspectLockableCurrency<T::AccountId> for Pallet<T> {
    // Correct function name and signature: balance_locked
    fn balance_locked(id: LockIdentifier, who: &T::AccountId) -> <Self as frame_support::traits::Currency<T::AccountId>>::Balance {
        log::warn!("InspectLockableCurrency::balance_locked called for {:?} with id {:?}", who, id);
        let mut total_locked: <Self as frame_support::traits::Currency<T::AccountId>>::Balance = Zero::zero();
        for lock in Locks::<T>::get(who) {
            total_locked = total_locked.saturating_add(lock.amount);
        }
        total_locked
    }
}

// --- Unbalanced Trait Implementation ---
impl<T: Config> fungible::Unbalanced<T::AccountId> for Pallet<T> {
    fn set_total_issuance(amount: Self::Balance) {
        TotalSupply::<T>::put(amount);
    }

    // Restored parameters
    fn decrease_balance(
        who: &T::AccountId,
        amount: Self::Balance,
        _precision: Precision,
        _preservation: Preservation,
        _fortitude: Fortitude,
    ) -> Result<Self::Balance, DispatchError> {
        log::warn!("Unimplemented decrease_balance called for {:?} amount {:?}", who, amount);
        Err(DispatchError::Unavailable)
    }

    // Restored parameters
    fn increase_balance(
        who: &T::AccountId,
        amount: Self::Balance,
        _precision: Precision,
    ) -> Result<Self::Balance, DispatchError> {
        log::warn!("Unimplemented increase_balance called for {:?} amount {:?}", who, amount);
        Err(DispatchError::Unavailable)
    }
    
    fn handle_dust(_dust: fungible::Dust<T::AccountId, Self>) {
        log::warn!("Unimplemented handle_dust called");
    }

    fn write_balance(
        who: &T::AccountId,
        amount: Self::Balance
    ) -> Result<Option<Self::Balance>, DispatchError> {
        log::warn!("Unimplemented write_balance called for {:?} amount {:?}", who, amount);
        let old_balance = Self::set_balance(who, amount); 
        Ok(Some(old_balance))
    }
} 

// --- Currency Trait Implementation ---
// This will be a more extensive implementation

// Define Imbalance types first (usually outside the main pallet module if they are generic)
// For now, let's define them simply within the scope they are used by the Currency impl.
// These are often unit structs if imbalances are handled immediately.

pub struct PositiveImbalance<T: Config>(BalanceOf<T>,
    #[doc(hidden)]
    core::marker::PhantomData<T>,
);
impl<T: Config> Default for PositiveImbalance<T> {
    fn default() -> Self {
        Self(Zero::zero(), core::marker::PhantomData)
    }
}
impl<T: Config> Drop for PositiveImbalance<T> {
    fn drop(&mut self) {
        // Can add a log if self.0 is not zero and it's unexpected
        // For now, keep it empty as per typical imbalance drop handling where it's assumed
        // the imbalance has been correctly handled or offset before dropping.
    }
}
impl<T: Config> PositiveImbalance<T> {
    fn new(amount: BalanceOf<T>) -> Self {
        PositiveImbalance(amount, PhantomData)
    }
}

pub struct NegativeImbalance<T: Config>(BalanceOf<T>,
    #[doc(hidden)]
    core::marker::PhantomData<T>,
);
impl<T: Config> Default for NegativeImbalance<T> {
    fn default() -> Self {
        Self(Zero::zero(), core::marker::PhantomData)
    }
}
impl<T: Config> Drop for NegativeImbalance<T> {
    fn drop(&mut self) {
        // Similar to PositiveImbalance, ensure it's empty or log
    }
}
impl<T: Config> NegativeImbalance<T> {
    fn new(amount: BalanceOf<T>) -> Self {
        NegativeImbalance(amount, PhantomData)
    }
}

// Required for Imbalance trait
impl<T: Config> frame_support::traits::TryDrop for PositiveImbalance<T> {
    fn try_drop(self) -> Result<(), Self> { Ok(()) } // Simplified
}
impl<T: Config> frame_support::traits::Imbalance<BalanceOf<T>> for PositiveImbalance<T> {
    type Opposite = NegativeImbalance<T>;
    fn zero() -> Self { PositiveImbalance(Zero::zero(), PhantomData) }
    fn drop_zero(self) -> Result<(), Self> { if self.0.is_zero() { Ok(()) } else { Err(self) } } // Corrected drop_zero
    fn split(self, amount: BalanceOf<T>) -> (Self, Self) { 
        let first = self.0.min(amount);
        (Self::new(first), Self::new(self.0.saturating_sub(first)))
    }
    fn merge(mut self, other: Self) -> Self { self.0 = self.0.saturating_add(other.0); self }
    fn subsume(&mut self, other: Self) { self.0 = self.0.saturating_add(other.0); }
    fn offset(self, other: Self::Opposite) -> frame_support::traits::SameOrOther<Self, Self::Opposite> { // Corrected return type
        let P = self.0;
        let N = other.0;
        if P >= N {
            frame_support::traits::SameOrOther::Same(Self::new(P.saturating_sub(N)))
        } else {
            frame_support::traits::SameOrOther::Other(NegativeImbalance::new(N.saturating_sub(P)))
        }
    }
    fn peek(&self) -> BalanceOf<T> { self.0 }
    fn extract(&mut self, amount: BalanceOf<T>) -> Self { // Added extract
        let taken = self.0.min(amount);
        self.0 = self.0.saturating_sub(taken);
        Self::new(taken)
    }
}

impl<T: Config> frame_support::traits::TryDrop for NegativeImbalance<T> {
    fn try_drop(self) -> Result<(), Self> { Ok(()) } // Simplified
}
impl<T: Config> frame_support::traits::Imbalance<BalanceOf<T>> for NegativeImbalance<T> {
    type Opposite = PositiveImbalance<T>;
    fn zero() -> Self { NegativeImbalance(Zero::zero(), PhantomData) }
    fn drop_zero(self) -> Result<(), Self> { if self.0.is_zero() { Ok(()) } else { Err(self) } } // Corrected drop_zero
    fn split(self, amount: BalanceOf<T>) -> (Self, Self) { 
        let first = self.0.min(amount);
        (Self::new(first), Self::new(self.0.saturating_sub(first)))
    }
    fn merge(mut self, other: Self) -> Self { self.0 = self.0.saturating_add(other.0); self }
    fn subsume(&mut self, other: Self) { self.0 = self.0.saturating_add(other.0); }
    fn offset(self, other: Self::Opposite) -> frame_support::traits::SameOrOther<Self, Self::Opposite> { // Corrected return type
        let N = self.0;
        let P = other.0;
        if N >= P {
            frame_support::traits::SameOrOther::Same(Self::new(N.saturating_sub(P)))
        } else {
            frame_support::traits::SameOrOther::Other(PositiveImbalance::new(P.saturating_sub(N)))
        }
    }
    fn peek(&self) -> BalanceOf<T> { self.0 }
    fn extract(&mut self, amount: BalanceOf<T>) -> Self { // Added extract
        let taken = self.0.min(amount);
        self.0 = self.0.saturating_sub(taken);
        Self::new(taken)
    }
}

use core::marker::PhantomData; // Ensure PhantomData is in scope
use frame_support::traits::tokens::{WithdrawReasons as CurrencyWithdrawReasons};

impl<T: Config> frame_support::traits::Currency<T::AccountId> for Pallet<T> 
where <T as pallet::Config>::Currency: frame_support::traits::Currency<<T as frame_system::Config>::AccountId> 
{
    type Balance = BalanceOf<T>;
    type PositiveImbalance = PositiveImbalance<T>; 
    type NegativeImbalance = NegativeImbalance<T>;

    fn total_issuance() -> Self::Balance {
        Pallet::<T>::total_supply()
    }

    fn minimum_balance() -> Self::Balance {
        Zero::zero() // pwRoko has no existential deposit concept separate from native
    }

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        Pallet::<T>::balances(who) // Total is same as free for this simple token
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        // Considering locks for free balance
        let balance = Pallet::<T>::balances(who);
        let locked = Pallet::<T>::locks(who).iter().map(|l| l.amount).fold(Zero::zero(), |acc: Self::Balance, bal: Self::Balance| acc.saturating_add(bal));
        balance.saturating_sub(locked)
    }

    fn ensure_can_withdraw(
        who: &T::AccountId,
        amount: Self::Balance,
        _reasons: CurrencyWithdrawReasons, // Mark unused
        _new_balance: Self::Balance, // Mark unused - current check is simpler
    ) -> DispatchResult {
        let current_free = Self::free_balance(who);
        ensure!(current_free >= amount, Error::<T>::InsufficientWrappedBalance);
        Ok(())
    }

    fn transfer(
        source: &T::AccountId,
        dest: &T::AccountId,
        value: Self::Balance,
        _existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        Self::do_transfer(source.clone(), dest.clone(), value)
    }

    fn withdraw(
        who: &T::AccountId,
        value: Self::Balance,
        _reasons: CurrencyWithdrawReasons, // Mark unused
        _liveness: ExistenceRequirement, // Mark unused
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        let current_free = Self::free_balance(who);
        ensure!(current_free >= value, Error::<T>::InsufficientWrappedBalance);
        Pallet::<T>::do_burn(who, value)?;
        Ok(NegativeImbalance::new(value))
    }

    fn deposit_creating(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::PositiveImbalance {
        // Minting pwRoko. This pallet doesn't have the same "account creation fee" concept as pallet-balances.
        // We assume the account exists if it receives tokens.
        match Pallet::<T>::do_mint(who, value) {
            Ok(_) => PositiveImbalance::new(value),
            Err(_) => PositiveImbalance::zero(), // Or handle error more explicitly if do_mint could fail here
        }
    }

    fn deposit_into_existing(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        Pallet::<T>::do_mint(who, value)?;
        Ok(PositiveImbalance::new(value))
    }
    
    fn make_free_balance_be(
        who: &T::AccountId,
        balance: Self::Balance
    ) -> frame_support::traits::SignedImbalance<Self::Balance, Self::PositiveImbalance> { 
        // Use fully qualified syntax for total_balance from the Currency trait itself
        let current_total = <Self as frame_support::traits::Currency<T::AccountId>>::total_balance(who);
        let current_locked = Pallet::<T>::locks(who).iter().map(|l| l.amount).fold(Zero::zero(), |acc: Self::Balance, bal: Self::Balance| acc.saturating_add(bal));
        let current_free = current_total.saturating_sub(current_locked);

        if balance > current_free {
            let métal = balance.saturating_sub(current_free);
            // Mint the difference to increase free balance.
            // This effectively increases total balance by `diff`.
            match Pallet::<T>::do_mint(who, métal) {
                Ok(_) => frame_support::traits::SignedImbalance::Positive(PositiveImbalance::new(métal)),
                Err(_) => frame_support::traits::SignedImbalance::Positive(PositiveImbalance::zero()), // Or handle error
            }
        } else if balance < current_free {
            let métal = current_free.saturating_sub(balance);
            // Burn the difference to decrease free balance.
            // This effectively decreases total balance by `diff`.
            match Pallet::<T>::do_burn(who, métal) {
                Ok(_) => frame_support::traits::SignedImbalance::Negative(NegativeImbalance::new(métal)), // Corrected: Negative expects NegativeImbalance
                Err(_) => frame_support::traits::SignedImbalance::Negative(NegativeImbalance::zero()), 
            }
        } else {
            // No change needed, return zero imbalance
            frame_support::traits::SignedImbalance::Positive(PositiveImbalance::zero())
        }
    }

    fn issue(amount: Self::Balance) -> Self::NegativeImbalance { // Added issue
        TotalSupply::<T>::mutate(|v| *v = v.saturating_add(amount));
        // Issuing new currency creates a negative imbalance (a debt from the system's perspective that needs to be credited somewhere)
        NegativeImbalance::new(amount)
    }

    fn burn(amount: Self::Balance) -> Self::PositiveImbalance { // Added burn
        TotalSupply::<T>::mutate(|v| *v = v.saturating_sub(amount));
        // Burning currency creates a positive imbalance (a credit to the system as supply is reduced)
        PositiveImbalance::new(amount)
    }

    // Placeholder for can_slash and slash as they are often used by staking
    fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
        Self::balances(who) >= value // Simplified: can slash if total balance is sufficient
    }

    fn slash(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        let actual_slashed = value.min(<Self as frame_support::traits::Currency<T::AccountId>>::total_balance(who));
        if let Err(_) = Pallet::<T>::do_burn(who, actual_slashed) {
            return (NegativeImbalance::zero(), <Self as frame_support::traits::Currency<T::AccountId>>::Balance::zero()); 
        }
        (NegativeImbalance::new(actual_slashed), actual_slashed)
    }

} 