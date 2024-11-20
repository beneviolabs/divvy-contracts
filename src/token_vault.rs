use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::AccountId;
use lazy_static::lazy_static;

// TODO should I never use std collections, or is this fine becuase its only use is in the lazy_static macro?
use std::collections::HashMap;

#[derive(BorshDeserialize, BorshSerialize)]
pub enum Token {
        // top two marketcap
        BTC,
        ETH,

        // Near native stable coins
        USDT,
        USDC,

        // AI coins
        NEAR,

        // High Marketcap L1s
        SOL,
}


// Define constants for token contract account IDs
const BTC_CONTRACT: &str = "btc-token.near";
const ETH_CONTRACT: &str = "eth-token.near";
const USDT_CONTRACT: &str = "usdt-token.near";
const USDC_CONTRACT: &str = "usdc-token.near";
const NEAR_CONTRACT: &str = "near-token.near";
const SOL_CONTRACT: &str = "sol-token.near";

lazy_static! {
    // Map of token contract account ID to token enum
    static ref TOKEN_MAP: HashMap<&'static str, Token> = {
        let mut m = HashMap::new();
        m.insert(BTC_CONTRACT, Token::BTC);
        m.insert(ETH_CONTRACT, Token::ETH);
        m.insert(USDT_CONTRACT, Token::USDT);
        m.insert(USDC_CONTRACT, Token::USDC);
        m.insert(NEAR_CONTRACT, Token::NEAR);
        m.insert(SOL_CONTRACT, Token::SOL);
        m
    };
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct TokenVault {
    // Type of token in the vault
    token_type: AccountId,
    // Total count of tokens
    total_assets: u128,
    // Total count of shares
    shares_total_supply: u128,
    // Shares of the vault by owner accountId.
    shares: LookupMap<AccountId, u128>,
    // Authorized users
    authorized_users: LookupMap<AccountId, bool>,
}

impl TokenVault {

    pub fn new(id: u32, token_type: AccountId, creator: AccountId) ->  TokenVault {
        assert!(TOKEN_MAP.contains_key(token_type.as_str()), "Token is not on the allowed list");
        let mut authorized_users = LookupMap::new(b"a".to_vec());
        authorized_users.insert(&creator, &true);
        Self {
            token_type,
            total_assets: 0,
            shares_total_supply: 0,
            shares: LookupMap::new(format!("s{}", id).into_bytes()),
            authorized_users,
        }
    }

    pub fn get_token_type(&self) -> AccountId {
        self.token_type.clone()
    }

    #[allow(dead_code)]
    pub fn authorize_user(&mut self, user: AccountId) {
        self.authorized_users.insert(&user, &true);
    }

    #[allow(dead_code)]
    fn deauthorize_user(&mut self, user: AccountId) {
        self.authorized_users.remove(&user);
    }

    fn assert_authorized(&self, caller: AccountId) {
        assert!(
            self.authorized_users.get(&caller).unwrap_or(false),
            "Caller is not authorized"
        );
    }

    fn calculate_share(&self, assets: u128) -> u128 {
        if self.total_assets == 0 || self.shares_total_supply == 0 {
            assets
        } else {
            assets * self.shares_total_supply / self.total_assets
        }
    }

    #[allow(dead_code)]
    pub fn preview_deposit(&self, sender: AccountId, assets: u128) -> u128 {
        self.assert_authorized(sender.clone());
        let sender_balance = self.shares.get(&sender).unwrap_or(0);
        self.calculate_share(assets) + sender_balance
    }

    pub fn add_liquidity(&mut self, sender: &AccountId, amount: u128) -> u128 {
        self.assert_authorized(sender.clone());
        // Calculate shares to mint based on net assets
        let shares = self.calculate_share(amount);

        // Update total assets and shares
        self.total_assets += amount;
        self.shares_total_supply += shares;

        // Update sender's balance
        let sender_balance = self.shares.get(&sender).unwrap_or(0);
        self.shares.insert(&sender, &(sender_balance + shares));

        near_sdk::env::log_str(format!("Sender: {}, Deposited {} assets, Minted {} shares", sender, amount, shares)
.as_str());
        shares
    }


    pub fn remove_liquidity(&mut self, sender: &AccountId, shares: u128) -> u128 {
        self.assert_authorized(sender.clone());
        let sender_balance: u128 = self.shares.get(&sender).unwrap_or(0);
        assert!(
            sender_balance >= shares,
            "Not enough shares to withdraw, balance: {}",
            sender_balance
        );

        let assets = self.total_assets * shares / self.shares_total_supply;

        // Update total assets and shares
        self.total_assets -= assets;
        self.shares_total_supply -= shares;

        // Update sender's balance
        let new_balance = sender_balance - shares;
        self.shares.insert(&sender, &new_balance);
        //if sender's balance is zero, deauthrozize the user
        if new_balance == 0 {
            self.deauthorize_user(sender.clone());
        }

        // Log the transaction
        near_sdk::env::log_str(&format!("Sender: {}, Withdrew {} shares, Burned {} assets", sender, shares, assets));

        assets
    }

}

#[cfg(test)]
mod tests {

    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    #[test]
    fn test_initialization() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "alice.near".parse().unwrap();
        let vault = TokenVault::new(0, BTC_CONTRACT.parse().unwrap(),  sender.clone());
        assert_eq!(vault.get_token_type(), "btc-token.near");
        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
    }

    #[test]
    fn test_deposit() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "roger.near".parse().unwrap();
        let mut vault = TokenVault::new(0, ETH_CONTRACT.parse().unwrap(),  sender.clone());

        assert_eq!(vault.get_token_type(), "eth-token.near");

        let shares = vault.add_liquidity(&sender, 10_000);
        assert_eq!(shares, 10_000);
        assert_eq!(vault.total_assets, 10_000);
        assert_eq!(vault.shares_total_supply, 10_000);
        assert_eq!(vault.shares.get(&sender).unwrap(), 10_000);
    }

    #[test]
    fn test_withdraw() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "phillipe.near".parse().unwrap();
        let mut vault = TokenVault::new(1, USDC_CONTRACT.parse().unwrap(),  sender.clone());

        vault.add_liquidity(&sender, 10_000);
        let assets = vault.remove_liquidity(&sender, 10_000);
        assert_eq!(assets, 10_000);
        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
        assert_eq!(vault.shares.get(&sender).unwrap(), 0);
    }

    #[test]
    fn test_authorization() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "alice.near".parse().unwrap();
        let mut vault = TokenVault::new(2, USDC_CONTRACT.parse().unwrap(), sender.clone());
        assert_eq!(vault.authorized_users.get(&sender).unwrap(), true);

        // Authorized user can deposit
        let shares = vault.add_liquidity(&sender, 10_000);
        assert_eq!(shares, 10_000);
    }

     #[test]
    fn test_deauthorize_user() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "alice.near".parse().unwrap();
        let mut vault = TokenVault::new(3, USDT_CONTRACT.parse().unwrap(),  sender.clone());

        assert!(vault.authorized_users.get(&sender).unwrap_or(false));

        vault.deauthorize_user(sender.clone());
        assert!(!vault.authorized_users.get(&sender).unwrap_or(false));
    }

    #[test]
    fn test_deauthorize_on_zero_balance() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "toy.near".parse().unwrap();
        let mut vault = TokenVault::new(3, USDT_CONTRACT.parse().unwrap(), sender.clone());

        vault.add_liquidity(&sender, 10_000);
        vault.remove_liquidity(&sender, 10_000);

        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
        assert_eq!(vault.shares.get(&sender).unwrap(), 0);

        assert_eq!(vault.authorized_users.get(&sender).unwrap_or(false), false);
    }

    #[test]
    fn test_multiple_deposits() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "phillipe.near".parse().unwrap();
        let mut vault = TokenVault::new(4, SOL_CONTRACT.parse().unwrap(), sender.clone());

        vault.add_liquidity(&sender, 5_000);
        vault.add_liquidity(&sender, 5_000);

        assert_eq!(vault.total_assets, 10_000);
        assert_eq!(vault.shares_total_supply, 10_000);
        assert_eq!(vault.shares.get(&sender).unwrap(), 10_000);
    }

    #[test]
    fn test_multiple_withdrawals() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let sender: AccountId = "root.near".parse().unwrap();
        let mut vault = TokenVault::new(4, NEAR_CONTRACT.parse().unwrap(),  sender.clone());


        vault.add_liquidity(&sender, 10_000);
        vault.remove_liquidity(&sender, 5_000);
        vault.remove_liquidity(&sender, 5_000);

        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
        assert_eq!(vault.shares.get(&sender).unwrap(), 0);
    }
}
