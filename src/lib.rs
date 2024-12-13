use near_contract_standards::fungible_token::Balance;
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::{env, near, AccountId, NearToken, PanicOnDefault, Promise, StorageUsage};
use stash::Stash;

mod token_vault;
mod stash;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
  stashes: UnorderedMap<u64, Stash>,
  accounts: UnorderedMap<AccountId, UnorderedSet<u64>>,
}


#[near]
impl Contract {

  #[init]
  pub fn new() -> Self {
    assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
    Self {
      stashes: UnorderedMap::new(b"s".to_vec()),
      accounts: UnorderedMap::new(b"a".to_vec()),
    }
  }

  //TODO impolement deposit and withdraw payable methods

  #[payable]
  pub fn create_stash(&mut self, name: String) {
    let prev_storage = env::storage_usage();
    let stash_id = self.stashes.len() as u64;
    self.stashes.insert(&stash_id, &Stash::new(stash_id, name));

    let mut set: UnorderedSet<u64> = self.accounts.get(&env::predecessor_account_id()).unwrap_or_else(|| UnorderedSet::new(b"s".to_vec()));
    set.insert(&stash_id);
    self.accounts.insert(&env::predecessor_account_id(), &set);

    self.internal_check_storage(prev_storage);

  }

  // add tokenVault into a stash
  pub fn add_token_to_stash(&mut self, stash_id: u64, token_id: AccountId) {
    let prev_storage = env::storage_usage();
    let mut stash = self.stashes.get(&stash_id).expect("ERR_STASH_NOT_FOUND");
    stash.add_vault(token_id);
    self.internal_check_storage(prev_storage);
  }

  // TODO swaps given amount_in of token_in into token_out
  pub fn deposit_swap(&mut self, _stash_id:u64, _token_in: AccountId, _token_out: AccountId, _amount_in: Balance, _min_amount_out: Balance) {

    // how to swap this via an agent and update stash.deposits
  }

  // add liquidity to a given stash
  pub fn add_liquidity_to_stash(&mut self, stash_id: u64, token_id: AccountId, amount: Balance) {
    let prev_storage = env::storage_usage();
    let mut stash = self.stashes.get(&stash_id).expect("ERR_STASH_NOT_FOUND");
    stash.add_liquidity(token_id, amount);
    self.internal_check_storage(prev_storage);
  }

  // remove liquidity from a given stash
  pub fn remove_liquidity_from_stash(&mut self, stash_id: u64, token_id: AccountId, amount: Balance) {
    let prev_storage = env::storage_usage();
    let mut stash = self.stashes.get(&stash_id).expect("ERR_STASH_NOT_FOUND");
    stash.remove_liquidity(token_id, amount);
    self.internal_check_storage(prev_storage);
  }

  // authorize additional stash contributor
  pub fn authorize_contributor(&mut self, stash_id: u64, account_id: AccountId) {
    let prev_storage = env::storage_usage();
    let mut stash = self.stashes.get(&stash_id).expect("ERR_STASH_NOT_FOUND");
    stash.authorize_contributor(account_id);
    self.internal_check_storage(prev_storage);
  }

  pub fn get_stashes_for_account(&self, account_id: AccountId) -> Vec<u64> {
    self.accounts.get(&account_id).unwrap_or_else(|| UnorderedSet::new(b"s".to_vec())).to_vec()
  }

 // TODO add helper methods to fetch shares per vault by accountId, decide what methods should be here vs in an indexer.

  #[payable]
  pub fn remove_stash(&mut self, stash_id: u64) {
    let prev_storage = env::storage_usage();
    self.stashes.remove(&stash_id);
    self.internal_check_storage(prev_storage);
  }

}

// internal methods
impl Contract {

  fn internal_check_storage(&self, prev_storage: StorageUsage) -> u128 {
      let storage_needed = env::storage_usage().checked_sub(prev_storage);
      let storage_cost = storage_needed.unwrap_or(0) as u128 * env::storage_byte_cost().as_yoctonear();

      let refund = env::attached_deposit()
          .checked_sub(NearToken::from_yoctonear(storage_cost))
          .expect(
              format!(
                  "ERR_STORAGE_DEPOSIT need {}, attatched {}",
                  storage_cost, env::attached_deposit()
              ).as_str()
          );
      if !refund.is_zero() {
          Promise::new(env::predecessor_account_id()).transfer(refund);
      }
      storage_cost
  }
}


#[cfg(test)]
mod tests {

    use near_sdk::{test_utils::{accounts, VMContextBuilder}, NearToken, testing_env};

    use super::*;

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
      let mut builder = VMContextBuilder::new();
      builder.predecessor_account_id(predecessor);
      builder
    }

    #[test]
    fn test_new_contract() {
      let context = get_context(accounts(0));
      testing_env!(context.build());
      let contract = Contract::new();
      assert!(contract.stashes.is_empty());
      assert!(contract.accounts.is_empty());
    }

    #[test]
    fn test_create_stash() {
      let mut context = get_context(accounts(0));
      testing_env!(context.attached_deposit(NearToken::from_near(1)).build());
      let mut contract = Contract::new();
      contract.create_stash("Roommates".to_string());
      assert_eq!(contract.stashes.len(), 1);
      assert_eq!(contract.accounts.len(), 1);
    }

    #[test]
    fn test_remove_stash() {
      let mut context = get_context(accounts(0));
      testing_env!(context.attached_deposit(NearToken::from_near(1)).build());
      let mut contract = Contract::new();
      contract.create_stash("Roommates".to_string());
      let stash_id = 0;
      contract.remove_stash(stash_id);
      assert!(contract.stashes.get(&stash_id).is_none());
    }
}

