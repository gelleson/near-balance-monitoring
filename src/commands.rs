use crate::cli::{Cli, Commands};
use crate::near::NearClient;
use crate::utils;
use crate::bot;
use std::time::Duration;
use tokio::time;

pub async fn run(cli: Cli) -> Result<(), String> {
    let near_client = NearClient::new();

    match cli.command {
        Commands::Balance { account_id } => {
            let balance = near_client.fetch_balance(&account_id).await?;
            print_balance(&account_id, balance);
        }
        Commands::Monitor {
            account_id,
            interval,
        } => {
            println!("Monitoring {account_id} every {interval}s...");
            let mut ticker = time::interval(Duration::from_secs(interval));
            let mut previous_balance: Option<u128> = None;

            loop {
                ticker.tick().await;
                match near_client.fetch_balance(&account_id).await {
                    Ok(balance) => {
                        let changed = previous_balance != Some(balance);
                        if changed {
                            print_balance(&account_id, balance);
                            previous_balance = Some(balance);
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}] Error: {e}", utils::now_timestamp());
                    }
                }
            }
        }
        Commands::Bot => {
            bot::run().await?;
        }
        Commands::Txs { account_id } => {
            let txs = near_client.fetch_transactions(&account_id).await?;
            if txs.is_empty() {
                println!("No transactions found for {account_id}");
            } else {
                println!("Last transactions for {account_id}:");
                for tx in txs {
                    println!("- Time:   {}\n  Hash:   {}\n  From:   {}\n  To:     {}\n  Amount: {}\n", 
                        utils::format_timestamp(tx.block_timestamp),
                        tx.hash, 
                        tx.signer_id, 
                        tx.receiver_id,
                        utils::format_near(tx.actions_agg.deposit as u128)
                    );
                }
            }
        }
    }
    Ok(())
}

fn print_balance(account_id: &str, balance: u128) {
    println!(
        "[{}] {} — {}",
        utils::now_timestamp(),
        account_id,
        utils::format_near(balance)
    );
}
