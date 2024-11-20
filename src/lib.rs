use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, NearToken, PanicOnDefault, Promise
};

use crate::token_vault::TokenVault;
use near_contract_standards::fungible_token::Balance;

mod token_vault;


/// Single swap action.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Vault which should be used for swapping.
    pub vault_id: u64,
    /// Token to swap from.
    pub token_in: AccountId,
    /// Amount to exchange.
    /// If amount_in is None, it will take amount_out from previous step.
    /// Will fail if amount_in is None on the first step.
    pub amount_in: Option<U128>,
    /// Token to swap into.
    pub token_out: AccountId,
    /// Required minimum amount of token_out.
    pub min_amount_out: U128,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    vaults: Vector<TokenVault>,
    /// Balances of deposited tokens for each account.
    deposited_amounts: LookupMap<AccountId, HashMap<AccountId, Balance>>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        Self {
            vaults: Vector::new(b"p".to_vec()),
            deposited_amounts: LookupMap::new(b"d".to_vec()),
        }
    }

    /// Adds new TokenVault with given token
    /// Attached NEAR should be enough to cover the added storage.
    #[payable]
    pub fn add_vault(&mut self, token: AccountId) -> u32 {
        self.internal_add_vault(TokenVault::new(
            self.vaults.len() as u32,
            token,
            env::predecessor_account_id(),
        ))
    }

    // invites another accountId to be an authorized contributor to the vault
    pub fn authorize_contributor(&mut self, vault_id: u64, contributor: AccountId) {
        let mut vault = self.vaults.get(vault_id).expect("ERR_NO_VAULT");
        vault.authorize_user(contributor);
        self.vaults.replace(vault_id, &vault);
    }

    //TODO use a virtual account here?
    // Add deposit associated to the predecessor's virtual account for the given token
    #[payable]
    pub fn deposit(&mut self, token_id: AccountId) {
        assert_one_yocto();
        let amount: Balance = env::attached_deposit().as_yoctonear();
        self.internal_deposit(&env::predecessor_account_id(), &token_id, amount);
    }


    /// Add liquidity from already deposited amounts to given pool.
    pub fn add_liquidity(&mut self, pool_id: u64, amount: u128) {
        let sender_id = env::predecessor_account_id();
        let mut pool = self.vaults.get(pool_id).expect("ERR_NO_POOL");
        let token = pool.get_token_type();

        let deposits = self.internal_get_deposits(&sender_id);
        let deposit = deposits.get(&token.clone()).unwrap_or(&0);
        assert!(*deposit >= amount, "ERR_NOT_ENOUGH");

        pool.add_liquidity(&sender_id, amount);
        self.vaults.replace(pool_id, &pool);

        // TODO - handle supported token types. The below asume the pool contains only near tokens
        Promise::new(env::current_account_id()).transfer(NearToken::from_near(amount));
    }

    /// Remove liquidity from the pool into general pool of liquidity.
    pub fn remove_liquidity(&mut self, pool_id: u64, shares: U128,) {
        let sender_id = env::predecessor_account_id();
        let mut pool = self.vaults.get(pool_id).expect("ERR_NO_POOL");
        let amount = pool.remove_liquidity(
            &sender_id,
            shares.into(),
        );
        self.vaults.replace(pool_id, &pool);
        let tokens = pool.get_token_type();
        let mut deposits = self.internal_get_deposits(&sender_id);
        *deposits.entry(tokens.clone()).or_default() += amount;
        self.deposited_amounts.insert(&sender_id, &deposits);
    }

    /// Withdraws given token from the deposits of given user.
    #[payable]
    pub fn withdraw(&mut self, token_id: AccountId, amount: U128) {
        assert_one_yocto();
        let amount: u128 = amount.into();
        let sender_id: AccountId = env::predecessor_account_id();
        let mut deposits: HashMap<AccountId, u128> = self.deposited_amounts.get(&sender_id).unwrap();
        let available_amount: u128 = deposits
            .get(&token_id)
            .expect("ERR_NO_TOKEN")
            .clone();
        println!("available_amount vs amount: {}, {}", available_amount, amount);
        assert!(available_amount >= amount, "ERR_NOT_ENOUGH");
        if available_amount == amount {
            deposits.remove(&token_id);
        } else {
            deposits.insert(token_id.clone(), available_amount - amount);
        }
        self.deposited_amounts.insert(&sender_id, &deposits);
        let receiver_id: AccountId = sender_id.try_into().unwrap();
         // TODO - handle supported token types. The below asume the pool contains only near tokens
        Promise::new(receiver_id).transfer(NearToken::from_near(amount));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use std::collections::HashMap;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
     fn test_withdraw_success() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_near(1)).build());

        let mut contract = Contract::new();
        let token_id: AccountId = "usdt-token.near".parse().unwrap();

        //add a new vault to the contract
        contract.add_vault(token_id.clone());
        let amount = 100;

        //simulate deposit
        contract.internal_deposit(&accounts(0), &token_id, amount);

        // Withdraw
        testing_env!(context.attached_deposit(NearToken::from_yoctonear(1)).build());
        contract.withdraw(token_id.clone(), U128(amount.into()));

        // Check balances
        let updated_deposits = contract.deposited_amounts.get(&accounts(0)).unwrap();
        assert_eq!(updated_deposits.get(&token_id), None);
    }

    #[test]
    #[should_panic(expected = "ERR_NOT_ENOUGH")]
    fn test_withdraw_insufficient_balance() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_yoctonear(1)).build());

        let mut contract = Contract::new();
        let token_id: AccountId = "usdt-token.near".parse().unwrap();
        let amount: u128 = 1000;

        // Simulate deposit
        let mut deposits: HashMap<AccountId, u128> = HashMap::new();
        deposits.insert(token_id.clone(), amount);
        contract.deposited_amounts.insert(&accounts(0), &deposits);

        // Attempt to withdraw more than available
        contract.withdraw(token_id.clone(), U128(amount + 1));
    }

    #[test]
    fn test_internal_add_vault_success() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_near(1)).build());

        let mut contract = Contract::new();
        let vault = TokenVault::new(0, "usdt-token.near".parse().unwrap(), "charles.near".parse().unwrap());

        let prev_storage = env::storage_usage();
        let id = contract.internal_add_vault(vault);
        let new_storage = env::storage_usage();

        assert_eq!(id, 0);
        assert!(new_storage > prev_storage);
    }

    #[test]
    #[should_panic(expected = "ERR_STORAGE_DEPOSIT")]
    fn test_internal_add_vault_insufficient_deposit() {
        let mut context = get_context(accounts(0));
        testing_env!(context.attached_deposit(NearToken::from_near(0)).build());

        let mut contract = Contract::new();
        let vault = TokenVault::new(0, "usdt-token.near".parse().unwrap(),"charles.near".parse().unwrap());

        contract.internal_add_vault(vault);
    }
}

/// Internal methods implementation.
impl Contract {
    /// Adds given pool to the list and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    fn internal_add_vault(&mut self, vault: TokenVault) -> u32 {
        let prev_storage = env::storage_usage();
        let id = self.vaults.len() as u32;
        self.vaults.push(&vault);
        assert!(
            (env::storage_usage() - prev_storage) as u128 * env::storage_byte_cost().as_yoctonear()
                <= env::attached_deposit().as_yoctonear(),
            "ERR_STORAGE_DEPOSIT"
        );
        id
    }

    // TODO Must we use virtual accounts?
    fn internal_deposit(
        &mut self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) {
        assert!(
            self.is_whitelisted_token(token_id),
            "{}",
            "Token is not on the allowed list"
        );
        let mut deposits = self.internal_get_deposits(sender_id);
        deposits.insert(token_id.clone(), amount + deposits.get(token_id).unwrap_or(&0));
        self.deposited_amounts.insert(sender_id, &deposits);
    }

    fn is_whitelisted_token(&self, token_id: &AccountId) -> bool {
        self.vaults.iter().any(|vault| vault.get_token_type() == *token_id)
    }

    /// Returns current balances across all tokens for given user.
    fn internal_get_deposits(&self, sender_id: &AccountId) -> HashMap<AccountId, Balance> {
        self.deposited_amounts
            .get(sender_id)
            .unwrap_or_else(|| HashMap::new())
            .clone()
    }
}
