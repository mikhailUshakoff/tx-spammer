use alloy::sol_types::SolCall;
use alloy::{
    dyn_abi::DynSolValue,
    network::{EthereumWallet, TransactionBuilder},
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::{BlockTransactionsKind, TransactionRequest},
    eips::BlockNumberOrTag,
    signers::local::PrivateKeySigner,
    sol,
};
use eyre::{Result, eyre};
use rand::Rng;
use std::str::FromStr;
use std::time::Duration;

sol! {
    function transfer(address to, uint256 amount) returns (bool);
    function balanceOf(address owner) view returns (uint256);
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Err(eyre!("missing command"));
    };

    match command.as_str() {
        "deploy" => {
            let contract = deploy().await?;
            println!("Contract deployed at: {contract}");
        }
        "spam" => {
            let token_address = parse_address_arg(args.next(), "token_address")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            spam(token_address, tps).await?;
        }
        "balance" => {
            let token_address = parse_address_arg(args.next(), "token_address")?;
            let owner_address = parse_address_arg(args.next(), "owner_address")?;
            let balance = erc20_balance(token_address, owner_address).await?;
            println!("Balance: {balance}");
        }
        "block" => {
            let block_number = parse_u64_arg(args.next(), "block_number")?;
            print_block(block_number).await?;
        }
        _ => {
            print_usage();
            return Err(eyre!("unknown command: {command}"));
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  tx-spammer deploy");
    eprintln!("  tx-spammer spam <token_address> <tps>");
    eprintln!("  tx-spammer balance <token_address> <owner_address>");
    eprintln!("  tx-spammer block <block_number>");
}

fn rpc_url() -> String {
    std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8547".to_string())
}

fn signer_and_from() -> Result<(PrivateKeySigner, Address)> {
    let private_key = std::env::var("PRIVATE_KEY")
        .map_err(|_| eyre!("PRIVATE_KEY environment variable not set"))?;
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let from = signer.address();
    Ok((signer, from))
}

async fn deploy() -> Result<Address> {
    let (signer, from) = signer_and_from()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::from(signer))
        .on_http(rpc_url().parse()?);

    let artifact = std::fs::read_to_string("out/MyToken.sol/MyToken.json")?;
    let artifact_json: serde_json::Value = serde_json::from_str(&artifact)?;
    let bytecode_hex = artifact_json["bytecode"]["object"]
        .as_str()
        .ok_or_else(|| eyre!("bytecode.object not found in artifact"))?;
    let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))?;

    let encoded_args = DynSolValue::Tuple(vec![
        DynSolValue::String("MyToken".to_string()),
        DynSolValue::String("MTK".to_string()),
        DynSolValue::Uint(U256::from(2_000_000_000u64), 256),
    ])
    .abi_encode_params();

    let mut deploy_data = bytecode;
    deploy_data.extend(encoded_args);

    println!("from: {from}");
    let deploy_tx = TransactionRequest::default()
        .from(from)
        .with_deploy_code(deploy_data);

    let receipt = provider
        .send_transaction(deploy_tx)
        .await?
        .get_receipt()
        .await?;
    receipt
        .contract_address
        .ok_or_else(|| eyre!("No contract address in receipt"))
}

async fn spam(contract: Address, tps: f64) -> Result<()> {
    if tps <= 0.0 {
        return Err(eyre!("tps must be greater than 0"));
    }

    let (signer, from) = signer_and_from()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::from(signer))
        .on_http(rpc_url().parse()?);

    println!("Spamming ERC20 transfers for {contract} at {tps} TPS...");

    let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / tps));
    let mut rng = rand::thread_rng();
    let mut count: u64 = 0;

    //let mut nonce = provider.get_transaction_count(from).await?;

    loop {
        interval.tick().await;

        let to = Address::from(rng.r#gen::<[u8; 20]>());
        let calldata = transferCall {
            to,
            amount: U256::from(1u64),
        }
        .abi_encode();

        let latest_block = provider
            .get_block_by_number(BlockNumberOrTag::Latest, BlockTransactionsKind::Hashes)
            .await?
            .ok_or_else(|| eyre!("latest block not found"))?;

        let mut base_fee = u128::from(latest_block.header.base_fee_per_gas.unwrap_or(25_000_000));
        if base_fee < 25_000_000 {
            base_fee = 25_000_000;
        }

        let max_priority_fee_per_gas = provider.get_max_priority_fee_per_gas().await?;
        let max_fee_per_gas = base_fee
            .saturating_mul(2)
            .saturating_add(max_priority_fee_per_gas);

        let tx = TransactionRequest::default()
            .from(from)
            .to(contract)
            .input(calldata.into())
            .with_max_fee_per_gas(max_fee_per_gas)
            .with_max_priority_fee_per_gas(max_priority_fee_per_gas);
        //tx.max_fee_per_gas = Some(max_fee_per_gas);
        //tx.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        //tx.nonce = Some(nonce);

        //nonce += 1;

        let _ = provider.send_transaction(tx).await?;

        count += 1;
        if count % 10000 == 0 {
            println!("Sent {count} transactions");
        }
    }
}

async fn erc20_balance(token: Address, owner: Address) -> Result<U256> {
    let provider = ProviderBuilder::new().on_http(rpc_url().parse()?);
    let calldata = balanceOfCall { owner }.abi_encode();
    let tx = TransactionRequest::default()
        .to(token)
        .input(calldata.into());
    let raw_result = provider.call(&tx).await?;
    let balance = balanceOfCall::abi_decode_returns(&raw_result, true)?._0;
    Ok(balance)
}

async fn print_block(block_number: u64) -> Result<()> {
    let provider = ProviderBuilder::new().on_http(rpc_url().parse()?);
    let block = provider
        .get_block_by_number(block_number.into(), BlockTransactionsKind::Hashes)
        .await?
        .ok_or_else(|| eyre!("block {block_number} not found"))?;

    println!("Block number: {block_number}");
    println!("Transactions count: {}", block.transactions.len());
    println!("Difficulty: {}", block.header.difficulty);
    println!("Gas used: {}", block.header.gas_used);
    println!("Gas limit: {}", block.header.gas_limit);
    println!("Blob gas used: {:?}", block.header.blob_gas_used);
    println!("Excess blob gas: {:?}", block.header.excess_blob_gas);
    println!("Base fee per gas: {:?}", block.header.base_fee_per_gas);
    
    Ok(())
}

fn parse_address_arg(value: Option<String>, name: &str) -> Result<Address> {
    let value = value.ok_or_else(|| eyre!("missing argument: {name}"))?;
    Address::from_str(&value).map_err(Into::into)
}

fn parse_f64_arg(value: Option<String>, name: &str) -> Result<f64> {
    let value = value.ok_or_else(|| eyre!("missing argument: {name}"))?;
    value.parse().map_err(Into::into)
}

fn parse_u64_arg(value: Option<String>, name: &str) -> Result<u64> {
    let value = value.ok_or_else(|| eyre!("missing argument: {name}"))?;
    value.parse().map_err(Into::into)
}
