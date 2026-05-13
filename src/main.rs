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
use alloy::eips::eip2718::Encodable2718;

sol! {
    function transfer(address to, uint256 amount) returns (bool);
    function balanceOf(address owner) view returns (uint256);
    function stressAdd(uint256 iters);
    function stressMulmod(uint256 iters);
    function stressKeccak(uint256 iters);
    function stressEcrecover(uint256 iters);
    function stressBlake2f(uint256 iters);
    function stressModexp(uint256 iters);
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
        "spam-batch" => {
            let token_address = parse_address_arg(args.next(), "token_address")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            spam_batch(token_address, tps).await?;
        }
        "deploy-zkgas-stress" => {
            let contract = deploy_zkgas_stress().await?;
            println!("ZkGasStress deployed at: {contract}");
        }
        "spam-modexp" => {
            let stress = parse_address_arg(args.next(), "stress_address")?;
            let iters = parse_u64_arg(args.next(), "iters")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            let calldata = stressModexpCall { iters: U256::from(iters) }.abi_encode();
            spam_calldata(stress, calldata, format!("stressModexp({iters})"), tps).await?;
        }
        "spam-add" => {
            let stress = parse_address_arg(args.next(), "stress_address")?;
            let iters = parse_u64_arg(args.next(), "iters")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            let calldata = stressAddCall { iters: U256::from(iters) }.abi_encode();
            spam_calldata(stress, calldata, format!("stressAdd({iters})"), tps).await?;
        }
        "spam-mulmod" => {
            let stress = parse_address_arg(args.next(), "stress_address")?;
            let iters = parse_u64_arg(args.next(), "iters")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            let calldata = stressMulmodCall { iters: U256::from(iters) }.abi_encode();
            spam_calldata(stress, calldata, format!("stressMulmod({iters})"), tps).await?;
        }
        "spam-keccak" => {
            let stress = parse_address_arg(args.next(), "stress_address")?;
            let iters = parse_u64_arg(args.next(), "iters")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            let calldata = stressKeccakCall { iters: U256::from(iters) }.abi_encode();
            spam_calldata(stress, calldata, format!("stressKeccak({iters})"), tps).await?;
        }
        "spam-ecrecover" => {
            let stress = parse_address_arg(args.next(), "stress_address")?;
            let iters = parse_u64_arg(args.next(), "iters")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            let calldata = stressEcrecoverCall { iters: U256::from(iters) }.abi_encode();
            spam_calldata(stress, calldata, format!("stressEcrecover({iters})"), tps).await?;
        }
        "spam-blake2f" => {
            let stress = parse_address_arg(args.next(), "stress_address")?;
            let iters = parse_u64_arg(args.next(), "iters")?;
            let tps = parse_f64_arg(args.next(), "tps")?;
            let calldata = stressBlake2fCall { iters: U256::from(iters) }.abi_encode();
            spam_calldata(stress, calldata, format!("stressBlake2f({iters})"), tps).await?;
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
    eprintln!("  tx-spammer spam-batch <token_address> <tps>");
    eprintln!("  tx-spammer deploy-zkgas-stress");
    eprintln!("  tx-spammer spam-add <stress_address> <iters> <tps>");
    eprintln!("  tx-spammer spam-mulmod <stress_address> <iters> <tps>");
    eprintln!("  tx-spammer spam-keccak <stress_address> <iters> <tps>");
    eprintln!("  tx-spammer spam-ecrecover <stress_address> <iters> <tps>");
    eprintln!("  tx-spammer spam-blake2f <stress_address> <iters> <tps>");
    eprintln!("  tx-spammer spam-modexp <stress_address> <iters> <tps>");
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

async fn deploy_zkgas_stress() -> Result<Address> {
    let (signer, from) = signer_and_from()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::from(signer))
        .on_http(rpc_url().parse()?);

    let artifact = std::fs::read_to_string("out/ZkGasStress.sol/ZkGasStress.json")?;
    let artifact_json: serde_json::Value = serde_json::from_str(&artifact)?;
    let bytecode_hex = artifact_json["bytecode"]["object"]
        .as_str()
        .ok_or_else(|| eyre!("bytecode.object not found in artifact"))?;
    let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))?;

    println!("from: {from}");
    let deploy_tx = TransactionRequest::default()
        .from(from)
        .with_deploy_code(bytecode);

    let receipt = provider
        .send_transaction(deploy_tx)
        .await?
        .get_receipt()
        .await?;
    receipt
        .contract_address
        .ok_or_else(|| eyre!("No contract address in receipt"))
}

async fn spam_calldata(
    stress: Address,
    calldata: Vec<u8>,
    label: String,
    tps: f64,
) -> Result<()> {
    if tps <= 0.0 {
        return Err(eyre!("tps must be greater than 0"));
    }

    let (signer, from) = signer_and_from()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::from(signer))
        .on_http(rpc_url().parse()?);

    println!("Spamming {label} on {stress} at {tps} TPS...");

    let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / tps));
    let mut count: u64 = 0;

    loop {
        interval.tick().await;

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
            .to(stress)
            .input(calldata.clone().into())
            .with_max_fee_per_gas(max_fee_per_gas)
            .with_max_priority_fee_per_gas(max_priority_fee_per_gas);

        let _ = provider.send_transaction(tx).await?;

        count += 1;
        if count % 1000 == 0 {
            println!("Sent {count} transactions");
        }
    }
}

async fn spam_batch(contract: Address, tps: f64) -> Result<()> {
    if tps <= 0.0 {
        return Err(eyre!("tps must be greater than 0"));
    }

    // Build these once, outside the loop
    let (signer, from) = signer_and_from()?;
    let wallet = EthereumWallet::from(signer);
    let client = reqwest::Client::new();
    let rpc = rpc_url();

    let provider = ProviderBuilder::new()
        .wallet(wallet.clone())
        .on_http(rpc.parse()?);

    println!("Spamming ERC20 transfers for {contract} at {tps} TPS (batched)...");

    let txs_per_second = tps.ceil() as u64;
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut rng = rand::thread_rng();
    let mut count: u64 = 0;

    let mut nonce = provider.get_transaction_count(from).await?;

    loop {
        interval.tick().await;
        let start = std::time::Instant::now();

        // Batch the fee-fetching RPC calls together with a manual JSON-RPC batch
        // so we pay only one round-trip instead of two sequential awaits.
        let fee_batch = serde_json::json!([
            {
                "jsonrpc": "2.0",
                "method": "eth_getBlockByNumber",
                "params": ["latest", false],
                "id": 1
            },
            {
                "jsonrpc": "2.0",
                "method": "eth_maxPriorityFeePerGas",
                "params": [],
                "id": 2
            }
        ]);

        let fee_results: Vec<serde_json::Value> = client
            .post(&rpc)
            .json(&fee_batch)
            .send()
            .await?
            .json()
            .await?;

        // Parse base fee from block result (id=1)
        let block_result = fee_results
            .iter()
            .find(|r| r["id"] == 1)
            .and_then(|r| r["result"].as_object())
            .ok_or_else(|| eyre!("missing block result"))?;

        let raw_base_fee = block_result
            .get("baseFeePerGas")
            .and_then(|v| v.as_str())
            .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(25_000_000);

        let base_fee = raw_base_fee.max(25_000_000);

        // Parse priority fee (id=2)
        let max_priority_fee_per_gas = fee_results
            .iter()
            .find(|r| r["id"] == 2)
            .and_then(|r| r["result"].as_str())
            .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(1_000_000_000); // 1 gwei fallback

        let max_fee_per_gas = base_fee
            .saturating_mul(2)
            .saturating_add(max_priority_fee_per_gas);

        // Build and sign all transactions, then pack into one batch request
        let mut tx_batch = Vec::with_capacity(txs_per_second as usize);

        for i in 0..txs_per_second {
            let to = Address::from(rng.r#gen::<[u8; 20]>());
            let calldata = transferCall {
                to,
                amount: U256::from(1u64),
            }
            .abi_encode();

            let tx = TransactionRequest::default()
                .from(from)
                .to(contract)
                .input(calldata.into())
                .with_nonce(nonce + i)
                .with_gas_limit(10_000_000)
                .with_chain_id(167011)
                .with_max_fee_per_gas(max_fee_per_gas)
                .with_max_priority_fee_per_gas(max_priority_fee_per_gas);

            let envelope = tx.build(&wallet).await?;
            let raw_tx = format!("0x{}", hex::encode(envelope.encoded_2718()));

            tx_batch.push(serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_sendRawTransaction",
                "params": [raw_tx],
                "id": i + 1
            }));
        }

        // Single HTTP request for all transactions
        let response = client
            .post(&rpc)
            .json(&tx_batch)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            eprintln!("Batch request failed with status {status}: {body}");
            // Don't adjust nonce blindly — re-fetch to get the true on-chain value
            nonce = provider.get_transaction_count(from).await?;
            continue;
        }

        let results: Vec<serde_json::Value> = response.json().await?;

        let mut sent = 0u64;
        let mut failed = 0u64;
        let mut highest_successful_nonce_offset: Option<u64> = None;

        for result in &results {
            let id = result["id"].as_u64().unwrap_or(0);
            if result.get("error").is_some() {
                eprintln!("Tx id={id} failed: {}", result["error"]);
                failed += 1;
            } else {
                sent += 1;
                // id is 1-based, so offset into nonce is id - 1
                highest_successful_nonce_offset = Some(
                    highest_successful_nonce_offset
                        .unwrap_or(0)
                        .max(id.saturating_sub(1)),
                );
            }
        }

        // Advance nonce past the highest tx that was accepted
        if let Some(offset) = highest_successful_nonce_offset {
            nonce += offset + 1;
        }

        // On any failure, re-sync from chain to avoid getting stuck
        if failed > 0 {
            eprintln!("{failed} txs failed — resyncing nonce from chain");
            nonce = provider.get_transaction_count(from).await?;
        }

        count += sent;

        let elapsed = start.elapsed().as_millis();
        println!("Sent {sent}/{txs_per_second} txs in {elapsed} ms");

        if count % 10_000 == 0 && count > 0 {
            println!("Total sent: {count} transactions");
        }
    }
}

async fn spam(contract: Address, tps: f64) -> Result<()> {
    if tps <= 0.0 {
        return Err(eyre!("tps must be greater than 0"));
    }

    let (signer, from) = signer_and_from()?;
    let provider = ProviderBuilder::new()
        //.with_recommended_fillers()
        .wallet(EthereumWallet::from(signer))
        .on_http(rpc_url().parse()?);

    println!("Spamming ERC20 transfers for {contract} at {tps} TPS...");

    let txs_per_second = tps.ceil() as u64;
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut rng = rand::thread_rng();
    let mut count: u64 = 0;

    // Fetch initial nonce once before the loop
    let mut nonce = provider.get_transaction_count(from).await?;

    loop {
        interval.tick().await;
        let start = std::time::Instant::now();
        // Fetch gas params ONCE per block, not per transaction
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

        // Fire all txs for this second concurrently, each with its own nonce
        let mut futures = Vec::with_capacity(txs_per_second as usize);

        for _ in 0..txs_per_second {
            let to = Address::from(rng.r#gen::<[u8; 20]>());
            let calldata = transferCall {
                to,
                amount: U256::from(1u64),
            }
            .abi_encode();

            let tx = TransactionRequest::default()
                .from(from)
                .to(contract)
                .input(calldata.into())
                .with_nonce(nonce)
                .with_gas_limit(10_000_000)
                .with_chain_id(167011)
                .with_max_fee_per_gas(max_fee_per_gas)
                .with_max_priority_fee_per_gas(max_priority_fee_per_gas);
            //println!("Prepared tx with nonce {}", nonce);
            nonce += 1;
            futures.push(provider.send_transaction(tx));
        }

        // Send all transactions in this batch concurrently
        let results = futures::future::join_all(futures).await;

        let mut sent = 0u64;
        let mut failed = 0u64;

        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(_) => sent += 1,
                Err(e) => {
                    eprintln!("Tx {i} failed: {e}");
                    failed += 1;
                }
            }
        }

        count += sent;

        if failed > 0 {
            eprintln!("Batch had {failed} failures, resyncing nonce...");
            nonce -= failed;
        }

        let elapsed = start.elapsed().as_millis();
        println!("Elapsed {elapsed} ms");

        if count % 10_000 == 0 {
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
