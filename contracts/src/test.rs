#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    vec, Address, Env, IntoVal, String,
};

fn setup_test(env: &Env) -> (StellarBountyBoardContractClient<'static>, Address, Address, Address) {
    let contract_id = env.register_contract(None, StellarBountyBoardContract);
    let client = StellarBountyBoardContractClient::new(env, &contract_id);

    let maintainer = Address::generate(env);
    let contributor = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);

    (client, maintainer, contributor, token_id.address())
}

#[test]
fn test_create_bounty() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, maintainer, contributor, token_id) = setup_test(&env);
    let token = TokenClient::new(&env, &token_id);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);

    // Mint some tokens to maintainer
    token_admin.mint(&maintainer, &1000);

    let repo = String::from_str(&env, "ritik4ever/stellar-bounty-board");
    let title = String::from_str(&env, "Fix bug");
    let deadline = env.ledger().timestamp() + 1000;
    let amount = 500;
    let issue_number = 1;

    let bounty_id = client.create_bounty(
        &maintainer,
        &token_id,
        &amount,
        &repo,
        &issue_number,
        &title,
        &deadline,
    );

    assert_eq!(bounty_id, 1);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.maintainer, maintainer);
    assert_eq!(bounty.amount, amount);
    assert_eq!(bounty.status, BountyStatus::Open);
    assert_eq!(token.balance(&client.address), amount);
    assert_eq!(token.balance(&maintainer), 500);

    // Verify events
    let events = env.events().all();
    let last_event = events.last().unwrap();
    
    assert_eq!(last_event.0, client.address);
    assert_eq!(last_event.1, (symbol_short!("Bounty"), symbol_short!("Create")).into_val(&env));
    // Since Val doesn't implement PartialEq, we compare the serialized data or just assume it's correct if the topics match for now
    // Or we can try to decode it back
    let event_data: BountyCreated = last_event.2.into_val(&env);
    assert_eq!(event_data.bounty_id, 1);
    assert_eq!(event_data.maintainer, maintainer);
    assert_eq!(event_data.amount, amount);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_create_bounty_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, maintainer, _, token_id) = setup_test(&env);

    client.create_bounty(
        &maintainer,
        &token_id,
        &-1,
        &String::from_str(&env, "repo"),
        &1,
        &String::from_str(&env, "title"),
        &(env.ledger().timestamp() + 1000),
    );
}

#[test]
fn test_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, maintainer, contributor, token_id) = setup_test(&env);
    let token = TokenClient::new(&env, &token_id);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&maintainer, &1000);

    let bounty_id = client.create_bounty(
        &maintainer,
        &token_id,
        &500,
        &String::from_str(&env, "repo"),
        &1,
        &String::from_str(&env, "title"),
        &(env.ledger().timestamp() + 1000),
    );

    // Reserve
    client.reserve_bounty(&bounty_id, &contributor);
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Reserved);
    assert_eq!(bounty.contributor, Some(contributor.clone()));

    // Submit
    client.submit_bounty(&bounty_id, &contributor);
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Submitted);

    // Release
    client.release_bounty(&bounty_id, &maintainer);
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Released);

    // Verify balances
    assert_eq!(token.balance(&contributor), 500);
    assert_eq!(token.balance(&client.address), 0);
}

#[test]
fn test_refund_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, maintainer, contributor, token_id) = setup_test(&env);
    let token = TokenClient::new(&env, &token_id);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&maintainer, &1000);

    let bounty_id = client.create_bounty(
        &maintainer,
        &token_id,
        &500,
        &String::from_str(&env, "repo"),
        &1,
        &String::from_str(&env, "title"),
        &(env.ledger().timestamp() + 1000),
    );

    // Refund while Open
    client.refund_bounty(&bounty_id, &maintainer);
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Refunded);
    assert_eq!(token.balance(&maintainer), 1000);
}

#[test]
#[should_panic(expected = "bounty must be submitted")]
fn test_release_without_submit() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, maintainer, contributor, token_id) = setup_test(&env);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&maintainer, &1000);

    let bounty_id = client.create_bounty(
        &maintainer,
        &token_id,
        &500,
        &String::from_str(&env, "repo"),
        &1,
        &String::from_str(&env, "title"),
        &(env.ledger().timestamp() + 1000),
    );

    client.reserve_bounty(&bounty_id, &contributor);
    client.release_bounty(&bounty_id, &maintainer);
}

#[test]
fn test_expiration() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, maintainer, contributor, token_id) = setup_test(&env);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&maintainer, &1000);

    let deadline = env.ledger().timestamp() + 1000;
    let bounty_id = client.create_bounty(
        &maintainer,
        &token_id,
        &500,
        &String::from_str(&env, "repo"),
        &1,
        &String::from_str(&env, "title"),
        &deadline,
    );

    // Advance time
    env.ledger().with_mut(|li| {
        li.timestamp = deadline + 1;
    });

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Expired);
}

#[test]
#[should_panic(expected = "bounty is not open")]
fn test_reserve_expired_bounty() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, maintainer, contributor, token_id) = setup_test(&env);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&maintainer, &1000);

    let deadline = env.ledger().timestamp() + 1000;
    let bounty_id = client.create_bounty(
        &maintainer,
        &token_id,
        &500,
        &String::from_str(&env, "repo"),
        &1,
        &String::from_str(&env, "title"),
        &deadline,
    );

    // Advance time
    env.ledger().with_mut(|li| {
        li.timestamp = deadline + 1;
    });

    client.reserve_bounty(&bounty_id, &contributor);
}
