use anyhow::{Result, anyhow};

/// Mock ICRC-1 transfer implementation
pub async fn icrc1_transfer(
    canister_id: &str,
    from_wallet: &str,
    to_wallet: &str,
    amount: f64,
) -> Result<()> {
    println!(
        "💸 MOCK TRANSFER: Canister: {}, From: {}, To: {}, Amount: {}",
        canister_id, from_wallet, to_wallet, amount
    );

    if from_wallet == to_wallet {
        return Err(anyhow!("Cannot transfer to the same wallet"));
    }

    if amount <= 0.0 {
        return Err(anyhow!("Amount must be greater than 0"));
    }

    Ok(())
}
