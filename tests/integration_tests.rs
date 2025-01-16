

use near_sdk::NearToken;
use near_workspaces::Account;
use near_workspaces::Contract;
use near_workspaces::network::Sandbox;
use near_workspaces::Worker;
use near_workspaces::Result;
use serde_json::json;


async fn setup_env() -> Result<(Worker<Sandbox>, Account, Contract)> {
    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;
    let wasm = include_bytes!("../target/wasm32-unknown-unknown/release/divvy_wealth.wasm");
    let contract = worker.dev_deploy(wasm).await?;
    Ok((worker, root, contract))
}

async fn init() -> Result<(Worker<Sandbox>, Account, Contract)> {
    let (worker, root, contract) = setup_env().await?;

    // initialize the contract
    let result = contract.call("new").transact().await?.into_result()?;
    assert!(result.outcome().is_success(), "Contract initialization failed");
    Ok((worker, root, contract))
}

#[tokio::test]
async fn test_create_stash() -> Result<()> {
    let (_worker, root, contract) = init().await?;

    // Create a stash
    let outcome = root
        .call(&contract.id(), "create_stash")
        .args_json(json!({"name": "Roommate slush funds"}))
        .deposit(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000))
        .transact()
        .await?;

    assert!(outcome.is_success());
    println!("root id is {}", &root.id());

    let args = &json!({"account_id": &root.id()});
    println!("args are {:#?}", args);

    // Check the stash was created
    let c = root.view(&contract.id(), "get_stashes_for_account")
        .args_json(args)
        .await?;
    c.logs.iter().for_each(|log| println!("{}", log));

    println!("c is {:#?}", c);

    let stashes: Vec<u64> = contract
        .view("get_stashes_for_account")
        .args_json(args)
        .await?
        .json()?;

    print!("stashes: {:#?}", stashes);

    assert_eq!(stashes.len(), 1);
    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_add_token_to_stash() -> Result<()> {
    // TODO remove worker if truuely unused
    let (_worker, root, contract) = setup_env().await?;

    // Create a stash
    let mut outcome  = root.call(&contract.id(), "create_stash")
        .args_json(serde_json::json!({"name": "Close Friends"}))
        .deposit(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000))
        .transact()
        .await?;

    println!("create_stash outcome: {:#?}", outcome);

    // Add a token to the stash
    outcome = root
        .call(&contract.id(), "add_token_to_stash")
        .args_json(serde_json::json!({"stash_id": 0, "token_id": "usdt.token.near"}))
        .transact()
        .await?;

    assert!(outcome.is_success());
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_remove_stash() -> Result<()> {
    let (_worker, root, contract) = setup_env().await?;

    // Create a stash
    let mut outcome = root.call(&contract.id(), "create_stash")
        .args_json(serde_json::json!({"name": "Roommates"}))
        .deposit(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000))
        .transact()
        .await?;


    println!("create_stash outcome: {:#?}", outcome);

    // Remove the stash
    outcome = root
        .call(&contract.id(),  "remove_stash")
        .args_json(serde_json::json!({"stash_id": 0}))
        .transact()
        .await?;

    assert!(outcome.is_success());

    // Check the stash was removed
    let stashes: Vec<u64> = contract
        .view("get_stashes_for_account")
        .args_json(serde_json::json!({"account_id": root.id()}))
        .await?
        .json()?;

    assert_eq!(stashes.len(), 0);
    Ok(())
}
