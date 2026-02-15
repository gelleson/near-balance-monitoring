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
                        let changed = previous_balance.map_or(true, |prev| prev != balance);
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
