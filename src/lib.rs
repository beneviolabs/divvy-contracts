use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

const BASIS_POINT_SCALE: u128 = 10_000;


#[derive(BorshDeserialize, BorshSerialize)]
#[derive(serde::Serialize)]
pub enum Token {
        // top two marketcap
        BTC,
        ETH,

        // stable coins
        USDT,
        USDC, //Near Native USDC

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

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TokenVault {
    exit_fee_basis_points: u128,
    entry_fee_basis_points: u128,

    // Type of token in the vault
    token_type: Token,
    // Total count of tokens
    total_assets: u128,
    // Total count of shares
    shares_total_supply: u128,
    // Shares of the vault by owner accountId.
    shares: LookupMap<AccountId, u128>,
}

#[near_bindgen]
impl TokenVault {
    #[init]
    pub fn new(entry_fee_basis_points: u128, exit_fee_basis_points: u128, token_type: Token
    ) -> Self {
        Self {
            exit_fee_basis_points,
            entry_fee_basis_points,
            token_type,
            total_assets: 0,
            shares_total_supply: 0,
            shares: LookupMap::new(b"b".to_vec()),
        }
    }

    pub fn get_exit_fee(&self) -> u128 {
        self.exit_fee_basis_points
    }

    pub fn preview_deposit(&self, assets: u128) -> u128 {
        let fee = self.calculate_fee(assets, self.entry_fee_basis_points);
        assets - fee
    }

    pub fn get_token_type(&self) -> String {
        self.token_type.as_str().to_string()
    }

    pub fn preview_mint(&self, shares: u128) -> u128 {
        let fee = self.calculate_fee(shares, self.entry_fee_basis_points);
        shares - fee
    }

    fn calculate_fee(&self, amount: u128, basis_points: u128) -> u128 {
        amount * basis_points / BASIS_POINT_SCALE
    }

    pub fn deposit(&mut self, sender: AccountId, receiver: AccountId, assets: u128) -> u128 {
        let fee = self.calculate_fee(assets, self.entry_fee_basis_points);
        let net_assets = assets - fee;

        // Calculate shares to mint based on net assets
        let shares = if self.total_assets == 0 || self.shares_total_supply == 0 {
            net_assets
        } else {
            net_assets * self.shares_total_supply / self.total_assets
        };

        // Update total assets and shares
        self.total_assets += net_assets;
        self.shares_total_supply += shares;

        // Update receiver's balance
        let receiver_balance = self.shares.get(&receiver).unwrap_or(0);
        self.shares.insert(&receiver, &(receiver_balance + shares));

        // Log the transaction
        env::log_str(format!("Sender: {}, Receiver: {}, Deposited {} assets, fee: {}, Minted {} shares", sender, receiver, net_assets, fee, shares).as_str());

        shares
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    #[test]
    fn test_deposit() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());

        let mut vault = TokenVault::new(100, 50, Token::ETH);
        let sender = accounts(0);
        let receiver = accounts(1);

        assert_eq!(vault.get_token_type(), "ETH");

        let shares = vault.deposit(sender.clone(), receiver.clone(), 10_000);
        assert_eq!(shares, 9_900);
        assert_eq!(vault.total_assets, 9_900);
        assert_eq!(vault.shares_total_supply, 9_900);
        assert_eq!(vault.shares.get(&receiver).unwrap(), 9_900);
    }
}
