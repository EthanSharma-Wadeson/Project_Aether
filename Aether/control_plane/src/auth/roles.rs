//! Reusable RBAC helpers for future write routes.
//!
//! Phase 1: primitives only — no production write handlers call these yet
//! (except tests). Hierarchy: viewer < operator < admin.

use crate::auth::middleware::AuthContext;
use crate::db::operators::OperatorRole;
use crate::error::{Error, Result};

/// Minimum role required for an action.
pub fn require_role(ctx: &AuthContext, minimum: OperatorRole) -> Result<()> {
    if role_rank(ctx.role) >= role_rank(minimum) {
        Ok(())
    } else {
        Err(Error::Forbidden(format!(
            "{} role required (have {})",
            minimum.as_str(),
            ctx.role.as_str()
        )))
    }
}

pub fn require_operator(ctx: &AuthContext) -> Result<()> {
    require_role(ctx, OperatorRole::Operator)
}

pub fn require_admin(ctx: &AuthContext) -> Result<()> {
    require_role(ctx, OperatorRole::Admin)
}

/// Viewer is read-only — any write-shaped check should fail for viewers.
pub fn reject_viewer_writes(ctx: &AuthContext) -> Result<()> {
    if ctx.role == OperatorRole::Viewer {
        return Err(Error::Forbidden("viewer role is read-only".into()));
    }
    Ok(())
}

fn role_rank(role: OperatorRole) -> u8 {
    match role {
        OperatorRole::Viewer => 0,
        OperatorRole::Operator => 1,
        OperatorRole::Admin => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(role: OperatorRole) -> AuthContext {
        AuthContext {
            operator_id: "op".into(),
            username: "u".into(),
            role,
        }
    }

    #[test]
    fn viewer_rejected_by_operator_requirement() {
        assert!(require_operator(&ctx(OperatorRole::Viewer)).is_err());
    }

    #[test]
    fn operator_accepted_by_operator_requirement() {
        assert!(require_operator(&ctx(OperatorRole::Operator)).is_ok());
    }

    #[test]
    fn admin_satisfies_operator_requirement() {
        assert!(require_operator(&ctx(OperatorRole::Admin)).is_ok());
    }

    #[test]
    fn operator_rejected_by_admin_requirement() {
        assert!(require_admin(&ctx(OperatorRole::Operator)).is_err());
    }
}
