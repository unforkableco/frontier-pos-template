#![cfg_attr(not(feature = "std"), no_std)]

// Move BalanceOf definition here to make it accessible outside the pallet module
pub type BalanceOf<T> = <<T as pallet::Config>::Currency as frame_support::traits::Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo; // Import WeightInfo
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::{ReservableCurrency}};
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;
    use sp_runtime::traits::{CheckedAdd, CheckedSub, Zero, Saturating};

    // TODO: Potentially import WeightInfo if benchmarks are added
    // use crate::WeightInfo;

    // Reference the top-level BalanceOf type
    // pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance; // Remove this line

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

        // TODO: Consider adding constants like MaxLocks or similar if needed.
    }

    // --- Storage --- 

    /// Stores the amount of native ROKO locked by each account.
    #[pallet::storage]
    #[pallet::getter(fn locked_balances)]
    pub type LockedBalances<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, crate::BalanceOf<T>, ValueQuery>;

    /// Stores the balance of the wrapped pwRoko token for each account.
    #[pallet::storage]
    #[pallet::getter(fn balances)]
    pub type Balances<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, crate::BalanceOf<T>, ValueQuery>;

    /// Stores the total supply of the wrapped pwRoko token.
    #[pallet::storage]
    #[pallet::getter(fn total_supply)]
    pub type TotalSupply<T: Config> = StorageValue<_, crate::BalanceOf<T>, ValueQuery>;

    /// Stores the ERC20-like allowances.
    #[pallet::storage]
    #[pallet::getter(fn allowances)]
    pub type Allowances<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, T::AccountId, // owner
        Blake2_128Concat, T::AccountId, // spender
        crate::BalanceOf<T>, // allowance
        ValueQuery,
    >;

    // --- Events --- 

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Native currency locked.
        Locked { who: T::AccountId, amount: crate::BalanceOf<T> },
        /// Native currency unlocked.
        Unlocked { who: T::AccountId, amount: crate::BalanceOf<T> },
        /// Wrapped token minted.
        Minted { to: T::AccountId, amount: crate::BalanceOf<T> },
        /// Wrapped token burned.
        Burned { from: T::AccountId, amount: crate::BalanceOf<T> },
        /// Wrapped token transferred.
        Transfer { from: T::AccountId, to: T::AccountId, amount: crate::BalanceOf<T> },
        /// Wrapped token allowance approved.
        Approval { owner: T::AccountId, spender: T::AccountId, amount: crate::BalanceOf<T> },
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

    // --- Extrinsics --- 

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Lock native ROKO tokens to mint wrapped pwRoko tokens.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::lock())]
        pub fn lock(origin: OriginFor<T>, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn unlock(origin: OriginFor<T>, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn transfer(origin: OriginFor<T>, to: T::AccountId, amount: crate::BalanceOf<T>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            Self::do_transfer(from, to, amount)
        }

        /// Approve a spender to withdraw wrapped pwRoko tokens from the caller's account.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::approve())]
        pub fn approve(origin: OriginFor<T>, spender: T::AccountId, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn transfer_from(origin: OriginFor<T>, from: T::AccountId, to: T::AccountId, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn do_mint(to: &T::AccountId, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn do_burn(from: &T::AccountId, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn do_transfer(from: T::AccountId, to: T::AccountId, amount: crate::BalanceOf<T>) -> DispatchResult {
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
        pub fn approve_spending(owner: T::AccountId, spender: T::AccountId, amount: crate::BalanceOf<T>) {
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