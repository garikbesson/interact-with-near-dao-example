use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::ext_contract;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, Gas, AccountId, Promise, PromiseError};

// const NFT_CONTRACT: &str = "x.paras.near";
const NFT_CONTRACT: &str = "paras-token-v2.testnet";
const NFT_MARKETPLACE_CONTRACT: &str = "paras-marketplace-v2.testnet";
const YOCTO_NEAR: u128 = 1;
const TGAS: u64 = 1_000_000_000_000;

// Define the contract structure
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
  nft_contract: AccountId,
  nft_marketplace_contract: AccountId
}

impl Default for Contract {
    // The default trait with which to initialize the contract
    fn default() -> Self {
        Self {
          nft_contract: NFT_CONTRACT.parse().unwrap(),
          nft_marketplace_contract: NFT_MARKETPLACE_CONTRACT.parse().unwrap(),
        }
    }
}

// Validator interface, for cross-contract calls
#[ext_contract(ext_nft_contract)]
trait ExternalNftContract {
  fn nft_token(&self, token_id: TokenId) -> Promise;
  fn nft_transfer(&self, receiver_id: AccountId, token_id: TokenId) -> Promise;
  fn nft_mint(&self, token_series_id: String, receiver_id: AccountId) -> Promise;
  fn buy(&self, nft_contract_id: AccountId, token_id: TokenId, ft_token_id: Option<AccountId>, price: Option<U128>) -> Promise;
}

// Implement the contract structure
#[near_bindgen]
impl Contract {
  pub fn nft_token(&self, token_id: TokenId) -> Promise {
    let promise = ext_nft_contract::ext(self.nft_contract.clone())
      .nft_token(token_id);

    return promise.then( // Create a promise to callback nft_token_callback
      Self::ext(env::current_account_id())
      .nft_token_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn nft_token_callback(&self, #[callback_result] call_result: Result<Token, PromiseError>) -> Option<Token> {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting NFT contract");
      return None;
    }

    // Return the token data
    let token_data: Token = call_result.unwrap();
    return Some(token_data);
  }

  // Before transfering token in a such way you may need to force user to approve your smart contract on NFT contract.
  #[payable]
  pub fn nft_transfer(&mut self, receiver_id: AccountId, token_id: TokenId) -> Promise {
    let promise = ext_nft_contract::ext(self.nft_contract.clone())
      .with_attached_deposit(YOCTO_NEAR)
      .nft_transfer(receiver_id, token_id);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .nft_transfer_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn nft_transfer_callback(&self, #[callback_result] call_result: Result<(), PromiseError>) {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting NFT contract");
    }
  }

  #[payable]
  pub fn nft_mint(&mut self, token_series_id: String, receiver_id: AccountId) -> Promise {
    let promise = ext_nft_contract::ext(self.nft_contract.clone())
      .with_static_gas(Gas(30*TGAS))
      .with_attached_deposit(env::attached_deposit())
      .nft_mint(token_series_id, receiver_id);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(30*TGAS))
      .nft_mint_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn nft_mint_callback(&self, #[callback_result] call_result: Result<TokenId, PromiseError>) -> Option<TokenId> {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting NFT contract");
      return None;
    }

    // Return the token data
    let token_id: TokenId = call_result.unwrap();
    return Some(token_id);
  }

  #[payable]
  pub fn buy(&mut self, nft_contract_id: AccountId, token_id: TokenId, ft_token_id: Option<AccountId>, price: Option<U128>) -> Promise {
    let promise = ext_nft_contract::ext(self.nft_marketplace_contract.clone())
      .with_static_gas(Gas(30*TGAS))
      .with_attached_deposit(env::attached_deposit())
      .buy(nft_contract_id, token_id, ft_token_id, price);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(30*TGAS))
      .buy_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn buy_callback(&self, #[callback_result] call_result: Result<(), PromiseError>) {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting NFT contract");
    }
  }
}
