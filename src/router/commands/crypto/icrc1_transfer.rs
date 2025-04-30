use anyhow::Result;
use super::icrc::icrc1::icrc1_transfer;

pub async fn icrc1_transfer_token(
    canister_id: &str,
    from: &str,
    to: &str,
    amount: f64
) -> Result<()> {
    println!("Transferring {} tokens from {} to {} on {}", amount, from, to, canister_id);
    icrc1_transfer(canister_id, from, to, amount).await?;
    Ok(())
}
