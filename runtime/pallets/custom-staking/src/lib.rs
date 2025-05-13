#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", warn(missing_docs))]
#![allow(deprecated)]

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, Get},
    transactional,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::StaticLookup;
use sp_runtime::Percent;
use sp_std::prelude::*;

// Import spécifique du pallet-staking au lieu d'utiliser glob import
pub use pallet_staking::{self as staking, BalanceOf, RewardDestination};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_staking::Config {
        /// Type pour les événements du pallet
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Paramètre personnalisé: Montant de bonus pour le staking (pourcentage 0-100)
        #[pallet::constant]
        type BonusPercentage: Get<u8>;
    }

    #[pallet::storage]
    #[pallet::getter(fn staker_bonuses)]
    pub type StakerBonuses<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Un bonus a été appliqué lors du staking
        /// [staker, amount, bonus]
        BonusApplied { staker: T::AccountId, amount: BalanceOf<T>, bonus: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// L'opération a échoué car la valeur est trop faible
        StakingAmountTooLow,
        /// Erreur provenant du pallet staking
        StakingError,
        /// Erreur de calcul de bonus 
        BonusCalculationError,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Version améliorée de la fonction bond du pallet staking original
        /// Ajoute un bonus de staking basé sur le paramètre BonusPercentage
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        #[transactional]
        pub fn enhanced_bond(
            origin: OriginFor<T>,
            _controller: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
            payee: RewardDestination<T::AccountId>,
        ) -> DispatchResult {
            let staker = ensure_signed(origin.clone())?;
            
            // Vérifie que le montant minimum est respecté
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::StakingAmountTooLow);
            
            // Calcule le bonus avec Percent pour éviter les problèmes de conversion
            let bonus_percent = Percent::from_percent(T::BonusPercentage::get());
            let bonus = bonus_percent.mul_floor(value);
            
            // Stocke le bonus pour cet utilisateur
            <StakerBonuses<T>>::insert(&staker, bonus);
            
            // Appelle la fonction originale du pallet-staking avec la logique standard
            pallet_staking::Pallet::<T>::bond(origin, value, payee)
                .map_err(|_| Error::<T>::StakingError)?;
            
            // Émet un événement pour le bonus
            Self::deposit_event(Event::BonusApplied { 
                staker, 
                amount: value,
                bonus,
            });
            
            Ok(())
        }
        
        /// Fonction utilitaire pour consulter le bonus actuel d'un staker
        #[pallet::call_index(1)]
        #[pallet::weight(5_000)]
        pub fn check_bonus(
            origin: OriginFor<T>,
            staker: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            
            let staker = T::Lookup::lookup(staker)?;
            let _ = Self::staker_bonuses(&staker);
            
            // Pas d'action spéciale - c'est juste pour consulter l'information
            // Les données sont accessibles via l'événement ou l'API RPC
            
            Ok(())
        }
    }
}

// Si vous activez la feature evm-precompiles, ce code s'active pour exposer votre fonction à l'EVM
#[cfg(feature = "evm-precompiles")]
pub mod precompile {
    use super::*;
    use fp_evm::{PrecompileHandle, PrecompileOutput};
    use pallet_evm::AddressMapping;
    use sp_std::marker::PhantomData;

    pub struct CustomStakingPrecompile<T>(PhantomData<T>);

    impl<T: pallet::Config + pallet_evm::Config> fp_evm::Precompile for CustomStakingPrecompile<T> {
        fn execute(handle: &mut impl PrecompileHandle) -> fp_evm::PrecompileResult {
            // Implémentation du précompile EVM pour les fonctions de staking
            // Ceci est un exemple simplifié
            let input = handle.input();
            
            // Les 4 premiers octets sont le sélecteur de fonction
            if input.len() < 4 {
                return Err(fp_evm::PrecompileFailure::Error { 
                    exit_status: fp_evm::ExitError::Other("Input too short".into()) 
                });
            }

            // Exemple d'implémentation - à compléter selon vos besoins
            Ok(PrecompileOutput {
                exit_status: fp_evm::ExitSucceed::Returned,
                output: vec![],
            })
        }
    }
} 