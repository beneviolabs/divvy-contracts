use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use lazy_static::lazy_static;

// TODO should I never use std collections, or is this fine becuase its only use is in the lazy_static macro?
use std::collections::HashMap;

use crate::token_vault;

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(serde::Serialize)]
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

impl Token {
    pub fn as_str(&self) -> &str {
        match self {
            Token::BTC => "BTC",
            Token::ETH => "ETH",
            Token::USDT => "USDT",
            Token::USDC => "USDC",
            Token::NEAR => "NEAR",
            Token::SOL => "SOL",
        }
    }
}

lazy_static! {
    // Map of token enum to token contract account id
    static ref TOKEN_MAP: HashMap<AccountId, Token> = {
       let mut m = HashMap::new();
       // TODO put valid token contract account ids
        m.insert("usdt.tether-token.near".parse().unwrap(), token_vault::Token::USDT);
        m.insert("usdc.circle-token.near".parse().unwrap(), token_vault::Token::USDC);
        m.insert("near.near".parse().unwrap(), token_vault::Token::NEAR);
        m.insert("sol.solana-token.near".parse().unwrap(), token_vault::Token::SOL);
        m.insert("eth.ethereum-token.near".parse().unwrap(), token_vault::Token::ETH);
        m.insert("btc.bitcoin-token.near".parse().unwrap(), token_vault::Token::BTC);
        m
    };
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
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

    pub fn new(token_type: AccountId) -> Self {
        assert!(TOKEN_MAP.contains_key(&token_type), "Token not supported");
        Self {
            token_type,
            total_assets: 0,
            shares_total_supply: 0,
            shares: LookupMap::new(b"b".to_vec()),
            authorized_users: LookupMap::new(b"a".to_vec()),
        }
    }

    pub fn get_token_type(&self) -> String {
        self.token_type.as_str().to_string()
    }

    pub fn authorize_user(&mut self, user: AccountId) {
        self.assert_authorized();
        self.authorized_users.insert(&user, &true);
    }

    pub fn deauthorize_user(&mut self, user: AccountId) {
        self.assert_authorized();
        self.authorized_users.remove(&user);
    }

    fn calculate_share(&self, assets: u128) -> u128 {
        if self.total_assets == 0 || self.shares_total_supply == 0 {
            assets
        } else {
            assets * self.shares_total_supply / self.total_assets
        }
    }
    pub fn preview_deposit(&self, sender: AccountId, assets: u128) -> u128 {
        self.assert_authorized();
        let sender_balance = self.shares.get(&sender).unwrap_or(0);
        self.calculate_share(assets) + sender_balance
    }

    pub fn deposit(&mut self, sender: AccountId, assets: u128) -> u128 {
        self.assert_authorized();
        // Calculate shares to mint based on net assets
        let shares = self.calculate_share(assets);

        // Transfer assets to the vault TODO, exit on failure

        // Update total assets and shares
        self.total_assets += assets;
        self.shares_total_supply += shares;

        // Update sender's balance
        let sender_balance = self.shares.get(&sender).unwrap_or(0);
        self.shares.insert(&sender, &(sender_balance + shares));

        env::log_str(format!("Sender: {}, Deposited {} assets, Minted {} shares", sender, assets, shares)
.as_str());
        shares
    }

    pub fn withdraw(&mut self, sender: AccountId, shares: u128) -> u128 {
        self.assert_authorized();
        let sender_balance: u128 = self.shares.get(&sender).unwrap_or(0);
        assert!(
            sender_balance >= shares,
            "Not enough shares to withdraw, balance: {}",
            sender_balance
        );

        // TODO Transfer assets to the sender, exit on failure

        let assets = self.total_assets * shares / self.shares_total_supply;

        // Update total assets and shares
        self.total_assets -= assets;
        self.shares_total_supply -= shares;

        // Update sender's balance
        self.shares.insert(&sender, &(sender_balance - shares));

        // Log the transaction
        env::log_str(&format!("Sender: {}, Withdrew {} shares, Burned {} assets", sender, shares, assets));

        assets
    }

     fn assert_authorized(&self) {
        let caller = env::predecessor_account_id();
        assert!(
            self.authorized_users.get(&caller).unwrap_or(false),
            "Caller is not authorized"
        );
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    #[test]
    fn test_initialization() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let vault = TokenVault::new("btc.bitcoin-token.near".parse().unwrap());
        assert_eq!(vault.get_token_type(), "BTC");
        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
    }

    #[test]
    fn test_deposit() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let mut vault = TokenVault::new("eth.ethereum-token.near".parse().unwrap());
        let sender = accounts(0);

        assert_eq!(vault.get_token_type(), "ETH");

        let shares = vault.deposit(sender.clone(), 10_000);
        assert_eq!(shares, 10_000);
        assert_eq!(vault.total_assets, 10_000);
        assert_eq!(vault.shares_total_supply, 10_000);
        assert_eq!(vault.shares.get(&sender).unwrap(), 10_000);
    }

    #[test]
    fn test_withdraw() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let mut vault = TokenVault::new("usdt.tether-token.near".parse().unwrap());
        let sender = accounts(0);

        vault.authorize_user(sender.clone());

        vault.deposit(sender.clone(), 10_000);
        let assets = vault.withdraw(sender.clone(), 10_000);
        assert_eq!(assets, 10_000);
        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
        assert_eq!(vault.shares.get(&sender).unwrap(), 0);
    }

    #[test]
    fn test_authorization() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let mut vault = TokenVault::new("usdc.circle-token.near".parse().unwrap());
        let sender = accounts(0);
        let unauthorized_user = accounts(1);

        vault.authorize_user(sender.clone());

        // Authorized user can deposit
        let shares = vault.deposit(sender.clone(), 10_000);
        assert_eq!(shares, 10_000);
        // TODO fix this
        //let result = std::panic::catch_unwind(|| vault.deposit(unauthorized_user.clone(), 10_000));
        //assert!(result.is_err(), "Unauthorized user should not be able to deposit");
    }

    #[test]
    fn test_multiple_deposits() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let mut vault = TokenVault::new("sol.solana-token.near".parse().unwrap());
        let sender = accounts(0);

        vault.authorize_user(sender.clone());

        vault.deposit(sender.clone(), 5_000);
        vault.deposit(sender.clone(), 5_000);

        assert_eq!(vault.total_assets, 10_000);
        assert_eq!(vault.shares_total_supply, 10_000);
        assert_eq!(vault.shares.get(&sender).unwrap(), 10_000);
    }

    #[test]
    fn test_multiple_withdrawals() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let mut vault = TokenVault::new("near.near".parse().unwrap());
        let sender = accounts(0);

        vault.authorize_user(sender.clone());

        vault.deposit(sender.clone(), 10_000);
        vault.withdraw(sender.clone(), 5_000);
        vault.withdraw(sender.clone(), 5_000);

        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.shares_total_supply, 0);
        assert_eq!(vault.shares.get(&sender).unwrap(), 0);
    }
}
