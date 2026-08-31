use serde_json::Value;

use super::errors::TreasuryWriteError;
use super::models::TreasuryOperation;

pub fn validate_dual_control_payload(
    op: TreasuryOperation,
    payload: &Value,
) -> Result<(), TreasuryWriteError> {
    match op {
        TreasuryOperation::AllocationCreate => {
            require_str(payload, "treasury_id")?;
            require_str(payload, "agent_id")?;
            require_str(payload, "asset_id")?;
            require_i64(payload, "ceiling_minor")?;
            let _ = payload.get("initial_minor");
            Ok(())
        }
        TreasuryOperation::AllocationIncrease => {
            require_str(payload, "allocation_id")?;
            require_i64(payload, "new_ceiling_minor")?;
            Ok(())
        }
        TreasuryOperation::AllocationDecrease => {
            require_str(payload, "allocation_id")?;
            require_i64(payload, "new_ceiling_minor")?;
            Ok(())
        }
        TreasuryOperation::Refund | TreasuryOperation::Chargeback | TreasuryOperation::Adjustment => {
            require_str(payload, "treasury_id")?;
            require_str(payload, "asset_id")?;
            require_i64(payload, "amount_minor")?;
            Ok(())
        }
        TreasuryOperation::Unfreeze => {
            require_str(payload, "treasury_id")?;
            Ok(())
        }
        other if other.requires_dual_control() => Ok(()),
        other => Err(TreasuryWriteError::BadRequest(format!(
            "operation {} is not a dual-control mutation request",
            other.as_str()
        ))),
    }
}

pub fn validate_single_shot(op: TreasuryOperation, payload: &Value) -> Result<(), TreasuryWriteError> {
    match op {
        TreasuryOperation::Reserve => {
            require_str(payload, "treasury_id")?;
            require_str(payload, "allocation_id")?;
            require_str(payload, "asset_id")?;
            require_i64(payload, "amount_minor")?;
            Ok(())
        }
        TreasuryOperation::Release | TreasuryOperation::SettlementPost => {
            require_str(payload, "reservation_id")?;
            Ok(())
        }
        TreasuryOperation::Freeze => {
            require_str(payload, "treasury_id")?;
            Ok(())
        }
        other => Err(TreasuryWriteError::BadRequest(format!(
            "operation {} is not single-shot",
            other.as_str()
        ))),
    }
}

fn require_str(payload: &Value, key: &str) -> Result<String, TreasuryWriteError> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| TreasuryWriteError::BadRequest(format!("missing {key}")))
}

fn require_i64(payload: &Value, key: &str) -> Result<i64, TreasuryWriteError> {
    payload
        .get(key)
        .and_then(|v| v.as_i64())
        .filter(|&n| n >= 0)
        .ok_or_else(|| TreasuryWriteError::BadRequest(format!("missing or invalid {key}")))
}
