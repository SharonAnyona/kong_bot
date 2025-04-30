use anyhow::Result;
use super::icrc::icrc2::icrc2_approve;

pub async fn icrc2_approve_token(
    canister_id: &str,
    from: &str,
    spender: &str,
    amount: f64
) -> Result<()> {
    // Use ICRC-2 approval logic (replace with real call)
    println!("Approving bot {} to spend {} from {}", spender, amount, from);
    icrc2_approve(canister_id, from, spender, amount).await?;
    Ok(())
}
