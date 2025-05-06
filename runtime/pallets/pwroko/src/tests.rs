#![cfg(test)]
use crate::{mock::*, Error, Event, Pallet as PwRoko, LockedBalances, Balances as PwRokoBalances, TotalSupply, Allowances};
use frame_support::{assert_ok, assert_noop};
use pallet_balances::Error as BalancesError;

#[test]
fn lock_works() {
    new_test_ext().execute_with(|| {
        let account = 1;
        let lock_amount = 1000;

        // Check initial state
        assert_eq!(PwRoko::locked_balances(account), 0);
        assert_eq!(PwRoko::balances(account), 0);
        assert_eq!(PwRoko::total_supply(), 0);
        assert_eq!(Balances::reserved_balance(account), 0);
        assert_eq!(Balances::free_balance(account), 10_000); 

        // Lock some ROKO
        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(account), lock_amount));

        // Check state after lock
        assert_eq!(PwRoko::locked_balances(account), lock_amount);
        assert_eq!(PwRoko::balances(account), lock_amount); // pwRoko balance increases
        assert_eq!(PwRoko::total_supply(), lock_amount);
        assert_eq!(Balances::reserved_balance(account), lock_amount); // Native ROKO reserved
        assert_eq!(Balances::free_balance(account), 9_000);

        // Check emitted events
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Minted { to: account, amount: lock_amount }));
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Locked { who: account, amount: lock_amount }));
        // Check Balances event
        System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Reserved { who: account, amount: lock_amount }));
    });
}

#[test]
fn lock_fails_insufficient_native_balance() {
    new_test_ext().execute_with(|| {
        let account = 3;
        let lock_amount = 1001;

        // Account 3 only has 100 free balance
        assert_noop!(
            PwRoko::lock(RuntimeOrigin::signed(account), lock_amount),
            BalancesError::<Test>::InsufficientBalance // Balances pallet returns this, mapped by our reserve call
            // We could map this in our pallet to Error::<Test>::InsufficientNativeBalance if desired
        );
    });
}

#[test]
fn unlock_works() {
    new_test_ext().execute_with(|| {
        let account = 1;
        let initial_lock = 1000;
        let unlock_amount = 300;

        // Initial lock
        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(account), initial_lock));
        assert_eq!(PwRoko::locked_balances(account), initial_lock);
        assert_eq!(PwRoko::balances(account), initial_lock);
        assert_eq!(PwRoko::total_supply(), initial_lock);
        assert_eq!(Balances::reserved_balance(account), initial_lock);
        assert_eq!(Balances::free_balance(account), 9_000); 

        // Unlock some
        assert_ok!(PwRoko::unlock(RuntimeOrigin::signed(account), unlock_amount));

        // Check state after unlock
        assert_eq!(PwRoko::locked_balances(account), initial_lock - unlock_amount);
        assert_eq!(PwRoko::balances(account), initial_lock - unlock_amount);
        assert_eq!(PwRoko::total_supply(), initial_lock - unlock_amount);
        assert_eq!(Balances::reserved_balance(account), initial_lock - unlock_amount); 
        assert_eq!(Balances::free_balance(account), 9_000 + unlock_amount); // Free balance increases

        // Check emitted events
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Burned { from: account, amount: unlock_amount }));
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Unlocked { who: account, amount: unlock_amount }));
        // Check Balances event
        System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved { who: account, amount: unlock_amount }));
    });
}

#[test]
fn unlock_fails_insufficient_wrapped_balance() {
    new_test_ext().execute_with(|| {
        let account = 1;
        let initial_lock = 500;
        let unlock_amount = 501;

        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(account), initial_lock));

        // Try to unlock more than available
        assert_noop!(
            PwRoko::unlock(RuntimeOrigin::signed(account), unlock_amount),
            Error::<Test>::InsufficientWrappedBalance
        );
    });
}

#[test]
fn transfer_works() {
    new_test_ext().execute_with(|| {
        let from_account = 1;
        let to_account = 2;
        let initial_lock = 1000;
        let transfer_amount = 400;

        // Lock funds for 'from' account
        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(from_account), initial_lock));
        assert_eq!(PwRoko::balances(from_account), initial_lock);
        assert_eq!(PwRoko::balances(to_account), 0);

        // Transfer pwRoko
        assert_ok!(PwRoko::transfer(RuntimeOrigin::signed(from_account), to_account, transfer_amount));

        // Check final balances
        assert_eq!(PwRoko::balances(from_account), initial_lock - transfer_amount);
        assert_eq!(PwRoko::balances(to_account), transfer_amount);
        assert_eq!(PwRoko::total_supply(), initial_lock); // Total supply unchanged

        // Check event
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Transfer { from: from_account, to: to_account, amount: transfer_amount }));
    });
}

#[test]
fn transfer_fails_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let from_account = 1;
        let to_account = 2;
        let initial_lock = 300;
        let transfer_amount = 301;

        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(from_account), initial_lock));
        assert_eq!(PwRoko::balances(from_account), initial_lock);

        assert_noop!(
            PwRoko::transfer(RuntimeOrigin::signed(from_account), to_account, transfer_amount),
            Error::<Test>::InsufficientWrappedBalance
        );
    });
}

#[test]
fn approve_works() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let spender = 2;
        let approve_amount = 500;

        assert_eq!(PwRoko::allowances(owner, spender), 0);

        // Approve spending
        assert_ok!(PwRoko::approve(RuntimeOrigin::signed(owner), spender, approve_amount));

        // Check allowance
        assert_eq!(PwRoko::allowances(owner, spender), approve_amount);

        // Check event
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Approval { owner, spender, amount: approve_amount }));

        // Approve again with different amount (overwrite)
        let new_approve_amount = 300;
        assert_ok!(PwRoko::approve(RuntimeOrigin::signed(owner), spender, new_approve_amount));
        assert_eq!(PwRoko::allowances(owner, spender), new_approve_amount);
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Approval { owner, spender, amount: new_approve_amount }));
    });
}

#[test]
fn transfer_from_works() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let spender = 2;
        let recipient = 4; // Needs some genesis balance for testing
        let initial_lock = 1000;
        let approve_amount = 500;
        let transfer_amount = 300;

        // Add genesis balance for recipient (mock doesn't have it)
        Balances::make_free_balance_be(&recipient, 1000);

        // Lock funds for owner
        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(owner), initial_lock));
        assert_eq!(PwRoko::balances(owner), initial_lock);
        assert_eq!(PwRoko::balances(recipient), 0);

        // Approve spender
        assert_ok!(PwRoko::approve(RuntimeOrigin::signed(owner), spender, approve_amount));
        assert_eq!(PwRoko::allowances(owner, spender), approve_amount);

        // Transfer from using allowance
        assert_ok!(PwRoko::transfer_from(RuntimeOrigin::signed(spender), owner, recipient, transfer_amount));

        // Check final balances and allowance
        assert_eq!(PwRoko::balances(owner), initial_lock - transfer_amount);
        assert_eq!(PwRoko::balances(recipient), transfer_amount);
        assert_eq!(PwRoko::allowances(owner, spender), approve_amount - transfer_amount);
        assert_eq!(PwRoko::total_supply(), initial_lock);

        // Check event (only Transfer, no Approval event for transfer_from)
        System::assert_has_event(RuntimeEvent::PwRoko(Event::Transfer { from: owner, to: recipient, amount: transfer_amount }));
    });
}

#[test]
fn transfer_from_fails_insufficient_allowance() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let spender = 2;
        let recipient = 4;
        let initial_lock = 1000;
        let approve_amount = 200;
        let transfer_amount = 300;

        Balances::make_free_balance_be(&recipient, 1000);
        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(owner), initial_lock));
        assert_ok!(PwRoko::approve(RuntimeOrigin::signed(owner), spender, approve_amount));

        // Try to transfer more than allowed
        assert_noop!(
            PwRoko::transfer_from(RuntimeOrigin::signed(spender), owner, recipient, transfer_amount),
            Error::<Test>::InsufficientAllowance
        );
    });
}

#[test]
fn transfer_from_fails_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let spender = 2;
        let recipient = 4;
        let initial_lock = 200; // Owner has less than transfer amount
        let approve_amount = 500;
        let transfer_amount = 300;

        Balances::make_free_balance_be(&recipient, 1000);
        assert_ok!(PwRoko::lock(RuntimeOrigin::signed(owner), initial_lock));
        assert_ok!(PwRoko::approve(RuntimeOrigin::signed(owner), spender, approve_amount));

        // Try to transfer more than owner has
        assert_noop!(
            PwRoko::transfer_from(RuntimeOrigin::signed(spender), owner, recipient, transfer_amount),
            Error::<Test>::InsufficientWrappedBalance
        );
    });
} 