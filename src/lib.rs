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
  fn swap(&self, pool_id: u64, token_in: AccountId, token_out: AccountId, amount_in: u128, min_amount_out: U128) -> Promise;
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
  pub fn external_call_callback(&self, #[callback_result] call_result: Result<(), PromiseError>) {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting external contract");
    }
  }

  #[payable]
  pub fn swap_tokens(&mut self, pool_id: u64, token_in: AccountId, token_out: AccountId, amount_in: u128, min_amount_out: U128) -> Promise {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR");

    let promise = ext_amm_contract::ext(self.amm_contract.clone())
      .with_static_gas(Gas(300*TGAS))
      .with_attached_deposit(YOCTO_NEAR)
      .swap(pool_id, token_in, token_out, amount_in, min_amount_out);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(30*TGAS))
      .external_call_callback()
    )
  }

  #[payable]
  pub fn call_with_attached_tokens(&mut self, receiver_id: AccountId, amount: U128) -> Promise {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR");

    let promise = ext(self.ft_contract.clone())
      .with_static_gas(Gas(150*TGAS))
      .with_attached_deposit(YOCTO_NEAR)
      .ft_transfer_call(receiver_id, amount, None, "".to_string());

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(100*TGAS))
      .external_call_callback()
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
