/** Role helpers for Enterprise Governance Console.
 *
 * Backend roles today: admin | operator | viewer.
 * Console personas map onto those without inventing new protocol roles:
 * - Administrator / Security Officer → admin
 * - Operator → operator
 * - Auditor / Viewer → viewer
 */

export type BackendRole = "admin" | "operator" | "viewer" | string | null;

export function roleLabel(role: BackendRole): string {
  switch (role) {
    case "admin":
      return "Administrator";
    case "operator":
      return "Operator";
    case "viewer":
      return "Viewer";
    default:
      return role ?? "Unknown";
  }
}

export function canWritePolicies(role: BackendRole): boolean {
  return role === "admin" || role === "operator";
}

export function canApprovePolicies(role: BackendRole): boolean {
  return role === "admin";
}

export function canPrepareApply(role: BackendRole): boolean {
  return role === "admin" || role === "operator";
}

export function canExecuteApply(role: BackendRole): boolean {
  return role === "admin";
}

export function canReconcile(role: BackendRole): boolean {
  return role === "admin";
}

export function canViewSecurityCentre(role: BackendRole): boolean {
  return role === "admin";
}

export function canAdminister(role: BackendRole): boolean {
  return role === "admin";
}
