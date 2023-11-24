use std::collections::HashMap;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_contract_standards::fungible_token::core::ext_ft_core::ext;
use near_sdk::ext_contract;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, serde_json, log, Gas, AccountId, Promise, PromiseOrValue, PromiseError};

const FT_CONTRACT: &str = "token-v3.cheddar.testnet";
const AMM_CONTRACT: &str = "v2.ref-finance.near";

const PRICE: u128 = 100_000_000_000_000_000_000_000;
const YOCTO_NEAR: u128 = 1;
const TGAS: u64 = 1_000_000_000_000;

// Define the contract structure
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
  ft_contract: AccountId,
  amm_contract: AccountId,
  price: U128
}

impl Default for Contract {
    // The default trait with which to initialize the contract
    fn default() -> Self {
        Self {
          ft_contract: FT_CONTRACT.parse().unwrap(),
          amm_contract: AMM_CONTRACT.parse().unwrap(),
          price: U128(PRICE),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PoolInfo {
  /// Pool kind.
  pub pool_kind: String,
  /// List of tokens in the pool.
  pub token_account_ids: Vec<AccountId>,
  /// How much NEAR this contract has.
  pub amounts: Vec<U128>,
  /// Fee charged for swap.
  pub total_fee: u32,
  /// Total number of shares.
  pub shares_total_supply: U128,
  pub amp: u64,
}

// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
  Action {
    // Parameters which you want to get in msg object, e.g. buyer_id
    buyer_id: Option<AccountId>,
  },
}

// Validator interface, for cross-contract calls
#[ext_contract(ext_amm_contract)]
trait ExternalAmmContract {
  fn swap(&self, actions: Vec<SwapAction>) -> Promise;
  fn get_pools(&self, from_index: u64, limit: u64) -> Promise;
  fn get_deposits(&self, account_id: AccountId) -> Promise;
}

// Implement the contract structure
#[near_bindgen]
impl Contract {
  #[payable]
  pub fn send_tokens(&mut self, receiver_id: AccountId, amount: U128) -> Promise {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR");

    let promise = ext(self.ft_contract.clone())
      .with_attached_deposit(YOCTO_NEAR)
      .ft_transfer(receiver_id, amount, None);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(30*TGAS))
      .external_call_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn external_call_callback(&self, #[callback_result] call_result: Result<String, PromiseError>) {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting external contract");
    }
  }

  #[payable]
  pub fn swap_tokens(&mut self, pool_id: u64, token_in: AccountId, token_out: AccountId, amount_in: U128, min_amount_out: U128) -> Promise {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR");

    let swap_action = SwapAction {
      pool_id,
      token_in,
      token_out,
      amount_in: Some(amount_in),
      min_amount_out
    };

    let mut actions = Vec::new();
    actions.push(swap_action);

    let promise = ext_amm_contract::ext(self.amm_contract.clone())
      .with_static_gas(Gas(150*TGAS))
      .with_attached_deposit(YOCTO_NEAR)
      .swap(actions);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(100*TGAS))
      .external_call_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn external_get_pools_callback(&self, #[callback_result] call_result: Result<Vec<PoolInfo>, PromiseError>) -> Option<Vec<PoolInfo>> {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting external contract");
      return None;
    }

    // Return the pools data
    let pools_data = call_result.unwrap();
    return Some(pools_data);
  }

  pub fn get_amm_pools(&self, from_index: u64, limit: u64) -> Promise {
    let promise = ext_amm_contract::ext(self.amm_contract.clone())
      .get_pools(from_index, limit);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .external_get_pools_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn external_get_deposits_callback(&self, #[callback_result] call_result: Result<HashMap<AccountId, U128>, PromiseError>) -> Option<HashMap<AccountId, U128>> {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting external contract");
      return None;
    }

    // Return the pools data
    let deposits_data = call_result.unwrap();
    return Some(deposits_data);
  }

  pub fn get_contract_deposits(&self) -> Promise {
    let promise = ext_amm_contract::ext(self.amm_contract.clone())
      .get_deposits(env::current_account_id());

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .external_get_deposits_callback()
    )
  }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
  // Callback on receiving tokens by this contract.
  // `msg` format is either "" for deposit or `TokenReceiverMessage`.
  fn ft_on_transfer(
    &mut self,
    sender_id: AccountId,
    amount: U128,
    msg: String,
  ) -> PromiseOrValue<U128> {
    let token_in = env::predecessor_account_id();

    assert!(token_in == self.ft_contract, "{}", "The token is not supported");
    assert!(amount >= self.price, "{}", "The attached amount is not enough");

    log!(format!("Sender id: {:?}", sender_id).as_str());

    if msg.is_empty() {
      // Your internal logic here
      PromiseOrValue::Value(U128(0))
    } else {
      let message =
        serde_json::from_str::<TokenReceiverMessage>(&msg).expect("WRONG_MSG_FORMAT");
      match message {
        TokenReceiverMessage::Action {
          buyer_id,
        } => {
          let buyer_id = buyer_id.map(|x| x.to_string());
          log!(format!("Target buyer id: {:?}", buyer_id).as_str());
          // Your internal business logic
          PromiseOrValue::Value(U128(0))
        }
      }
    }
  }
}
