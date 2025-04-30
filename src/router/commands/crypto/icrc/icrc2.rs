use anyhow::{Result, anyhow};

/// Mock ICRC-2 approve implementation
pub async fn icrc2_approve(
    canister_id: &str,
    from_wallet: &str,
    spender: &str,
    amount: f64,
) -> Result<()> {
    println!(
        "✅ MOCK APPROVE: Canister: {}, From Wallet: {}, Spender: {}, Amount: {}",
        canister_id, from_wallet, spender, amount
    );

    // Simulate a successful approval
    if amount <= 0.0 {
        return Err(anyhow!("Amount must be greater than 0"));
    }

    Ok(())
}
