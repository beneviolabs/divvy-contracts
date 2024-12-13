use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, AccountId, NearToken, PanicOnDefault, Promise
};
use near_contract_standards::fungible_token::Balance;

use crate::token_vault::TokenVault;

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Stash {
    id: u64,
    name: String,
    vaults: LookupMap<AccountId, TokenVault>,
    /// Balances of deposited tokens for each account.
    deposited_amounts: LookupMap<AccountId, UnorderedMap<AccountId, Balance>>,
    // Authorized users
    authorized_users: LookupMap<AccountId, bool>,
}

#[allow(dead_code)] //TODO
impl Stash {
    pub fn new(id: u64, name: String) -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        let mut authorized_users = LookupMap::new(b"a".to_vec());
        authorized_users.insert(&env::predecessor_account_id(), &true);
        Self {
            id,
            name,
            vaults: LookupMap::new(b"v".to_vec()),
            deposited_amounts: LookupMap::new(b"d".to_vec()),
            authorized_users,
        }
    }

    /// Adds new TokenVault with given token
    /// Attached NEAR should be enough to cover the added storage.
    pub fn add_vault(&mut self, token: AccountId) {
        self.internal_add_vault(TokenVault::new(token))
    }

    // invites another accountId to be an authorized contributor to the vault
    pub fn authorize_contributor(&mut self, user: AccountId) {
        self.authorized_users.insert(&user, &true);
    }

    fn assert_authorized(&self, caller: AccountId) {
        assert!(
            self.authorized_users.get(&caller).unwrap_or(false),
            "Caller is not authorized"
        );
    }

    // TODO use a virtual account here?
    // Add deposit associated to the predecessor's virtual account for the given token
    pub fn deposit(&mut self, token_id: AccountId) -> Balance {
        let sender = env::predecessor_account_id();
        self.assert_authorized(sender.clone());
        let amount: Balance = env::attached_deposit().as_yoctonear();
        self.internal_deposit(&sender, &token_id, amount)
    }


    /// Add liquidity from already deposited amounts to given Stash.
    pub fn add_liquidity(&mut self, token_id:AccountId, amount: u128) -> u128 {
        let sender_id = env::predecessor_account_id();
        self.assert_authorized(sender_id.clone());
        let mut stash = self.vaults.get(&token_id).expect("ERR_NO_Stash");
        let token = stash.get_token_type();

        let deposits = self.internal_get_deposits(&sender_id);
        let deposit = deposits.get(&token.clone()).unwrap_or(0);
        assert!(deposit >= amount, "ERR_NOT_ENOUGH");

        let shares = stash.add_liquidity(&sender_id, amount);
        self.vaults.insert(&token_id, &stash);

        // TODO - handle supported token types. The below assumes the Stash contains only near tokens
        //Promise::new(env::current_account_id()).transfer(NearToken::from_near(amount));
        shares
    }

    /// Remove liquidity from the Stash into the user's deposits
    pub fn remove_liquidity(&mut self, token_id:AccountId, shares: u128,) -> u128 {
        let sender_id = env::predecessor_account_id();
        self.assert_authorized(sender_id.clone());
        let mut stash = self.vaults.get(&token_id).expect("ERR_NO_Stash");
        let new_balance = stash.remove_liquidity(
            &sender_id,
            shares.into(),
        );
        self.vaults.insert(&token_id, &stash);
        let tokens = stash.get_token_type();
        let mut deposits = self.internal_get_deposits(&sender_id);
        let current_balance = deposits.get(&tokens).unwrap_or(0);
        deposits.insert(&tokens, &(current_balance + new_balance));
        self.deposited_amounts.insert(&sender_id, &deposits);

        new_balance
    }

    /// Withdraws given token from the deposits of given user.
    pub fn withdraw(&mut self, token_id: AccountId, amount: U128) {
        assert_one_yocto();
        let amount: u128 = amount.into();
        let sender_id: AccountId = env::predecessor_account_id();
        self.assert_authorized(sender_id.clone());
        let mut deposits: UnorderedMap<AccountId, u128> = self.deposited_amounts.get(&sender_id).unwrap();
        let available_amount: u128 = deposits
            .get(&token_id)
            .expect("ERR_NO_TOKEN")
            .clone();
        println!("available_amount vs amount: {}, {}", available_amount, amount);
        assert!(available_amount >= amount, "ERR_NOT_ENOUGH");
        if available_amount == amount {
            deposits.remove(&token_id);

            //if sender's balance is zero, deauthrozize the user
            if deposits.is_empty() {
                self.authorized_users.remove(&sender_id);
            }
        } else {
            deposits.insert(&token_id.clone(), &(available_amount - amount));
        }
        self.deposited_amounts.insert(&sender_id, &deposits);


        let receiver_id: AccountId = sender_id.try_into().unwrap();
         // TODO - handle supported token types. The below assumes the Stash contains only near tokens
        Promise::new(receiver_id).transfer(NearToken::from_near(amount));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
     fn test_withdraw_success() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_near(1)).build());

        let mut contract = Stash::new(1, "501c3 donations for 2025".to_string());
        let token_id: AccountId = "usdt-token.near".parse().unwrap();

        //add a new vault to the Stash
        contract.add_vault(token_id.clone());
        let amount = 100;

        //simulate deposit
        contract.internal_deposit(&accounts(0), &token_id, amount);

        // Withdraw
        testing_env!(context.attached_deposit(NearToken::from_yoctonear(1)).build());
        contract.withdraw(token_id.clone(), U128(amount.into()));

        // Check balances
        let updated_deposits = contract.deposited_amounts.get(&accounts(0)).unwrap();
        assert_eq!(updated_deposits.get(&accounts(0)), None);
        assert_eq!(contract.authorized_users.get(&accounts(0)), None);
    }
    #[test]
    #[should_panic(expected = "ERR_NOT_ENOUGH")]
    fn test_withdraw_insufficient_balance() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_yoctonear(1)).build());

        let mut contract = Stash::new(1, "Weekend getaway to Miami".to_string());
        let token_id: AccountId = "usdt-token.near".parse().unwrap();
        let amount: u128 = 1000;

        // Simulate deposit
        let mut deposits: UnorderedMap<AccountId, u128> = UnorderedMap::new(b"d".to_vec());
        deposits.insert(&token_id.clone(), &amount);
        contract.deposited_amounts.insert(&accounts(0), &deposits);

        // Attempt to withdraw more than available
        contract.withdraw(token_id.clone(), U128(amount + 1));
    }

    #[test]
    fn test_internal_add_vault_success() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_near(1)).build());

        let mut contract = Stash::new(1, "Weekend getaway to Miami".to_string());
        let vault = TokenVault::new("usdt-token.near".parse().unwrap());
        let token_type = vault.get_token_type();

        let prev_storage = env::storage_usage();
        contract.internal_add_vault(vault);
        let new_storage = env::storage_usage();

        assert!(contract.vaults.get(&token_type).is_some(), "Vault was not added");
        assert!(new_storage > prev_storage);
    }

    #[test]
    #[should_panic(expected = "ERR_STORAGE_DEPOSIT")]
    fn test_internal_add_vault_insufficient_deposit() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_near(0)).build());

        let mut contract = Stash::new(1, "A week in Barcelona".to_string());
        let vault = TokenVault::new("usdt-token.near".parse().unwrap());

        contract.internal_add_vault(vault);
    }

    #[test]
    fn test_authorization() {
        let sender: AccountId = "alice.near".parse().unwrap();
        let mut context = get_context(sender.clone());
        testing_env!(context.attached_deposit(NearToken::from_near(1)).build());

        let mut stash = Stash::new(1, "A week in Barcelona".to_string());
        let vault = TokenVault::new("usdt-token.near".parse().unwrap());

        assert_eq!(stash.authorized_users.get(&sender).unwrap(), true);
        stash.internal_add_vault(vault);

        testing_env!(context.attached_deposit(NearToken::from_near(100)).build());
        // Authorized user can deposit
        let shares= stash.deposit("usdt-token.near".parse().unwrap());
        assert_eq!(shares, 100000000000000000000000000);
    }
}

/// Internal methods implementation.
impl Stash {
    /// Adds given Stash to the list and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    fn internal_add_vault(&mut self, vault: TokenVault) {
        let prev_storage = env::storage_usage();

        self.vaults.insert(&vault.get_token_type(), &vault);
        assert!(
            (env::storage_usage() - prev_storage) as u128 * env::storage_byte_cost().as_yoctonear()
                <= env::attached_deposit().as_yoctonear(),
            "ERR_STORAGE_DEPOSIT"
        );
    }

    // TODO Must we use virtual accounts?
    fn internal_deposit(
        &mut self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) -> Balance {
        assert!(
            self.is_allowlisted_token(token_id),
            "{}",
            "Token is not on the allowed list"
        );
        let mut deposits = self.internal_get_deposits(sender_id);
        deposits.insert(&token_id.clone(), &(amount + deposits.get(token_id).unwrap_or(0)));
         self.deposited_amounts.insert(sender_id, &deposits);
        deposits.get(token_id).unwrap().clone()
    }

    fn is_allowlisted_token(&self, token_id: &AccountId) -> bool {
        self.vaults.contains_key(token_id)
    }

    /// Returns current balances across all tokens for given user.
    fn internal_get_deposits(&self, sender_id: &AccountId) -> UnorderedMap<AccountId, Balance> {
        self.deposited_amounts
            .get(sender_id)
            .unwrap_or_else(|| UnorderedMap::new(b"d".to_vec()))
    }
}
