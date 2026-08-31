import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { useAuthStore } from "../store/auth";
import { useUiStore } from "../store/ui";
import {
  canAdminister,
  canReconcile,
  canViewSecurityCentre,
  roleLabel,
} from "../lib/roles";
import { ApplyDisabledBanner } from "./ui";

function linkClass({ isActive }: { isActive: boolean }) {
  return isActive ? "active" : "";
}

export function Layout() {
  const navigate = useNavigate();
  const { role, clear } = useAuthStore();
  const { theme, toggleTheme } = useUiStore();

  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="brand">
          <h1>Aether</h1>
          <p className="muted">Enterprise Governance Console</p>
        </div>

        <nav aria-label="Primary">
          <div className="nav-label">Overview</div>
          <div className="nav-group">
            <NavLink to="/dashboard" className={linkClass}>
              Dashboard
            </NavLink>
          </div>

          <div className="nav-label">Protocol</div>
          <div className="nav-group">
            <NavLink to="/agents" className={linkClass}>
              Agents
            </NavLink>
            <NavLink to="/capabilities" className={linkClass}>
              Capabilities
            </NavLink>
            <NavLink to="/economic" className={linkClass}>
              Escrow
            </NavLink>
            <NavLink to="/settlements" className={linkClass}>
              Settlements
            </NavLink>
            <NavLink to="/reputation" className={linkClass}>
              Reputation
            </NavLink>
          </div>

          <div className="nav-label">Governance</div>
          <div className="nav-group">
            <NavLink to="/policies" className={linkClass}>
              Policies
            </NavLink>
            <NavLink to="/apply" className={linkClass}>
              Apply Workflows
            </NavLink>
            <NavLink to="/treasury" className={linkClass}>
              Treasury
            </NavLink>
            <NavLink to="/audit" className={linkClass}>
              Audit Explorer
            </NavLink>
          </div>

          {(canViewSecurityCentre(role) || canAdminister(role)) && (
            <>
              <div className="nav-label">Operations</div>
              <div className="nav-group">
                {canViewSecurityCentre(role) && (
                  <NavLink to="/security" className={linkClass}>
                    Security Centre
                  </NavLink>
                )}
                {canReconcile(role) && (
                  <NavLink to="/reconcile" className={linkClass}>
                    Reconcile
                  </NavLink>
                )}
                {canAdminister(role) && (
                  <NavLink to="/admin" className={linkClass}>
                    System Admin
                  </NavLink>
                )}
              </div>
            </>
          )}
        </nav>

        <div className="sidebar-footer">
          <p className="muted">Role: {roleLabel(role)}</p>
          <div className="button-row">
            <button type="button" className="secondary" onClick={toggleTheme}>
              {theme === "dark" ? "Light mode" : "Dark mode"}
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => {
                clear();
                navigate("/login");
              }}
            >
              Sign out
            </button>
          </div>
        </div>
      </aside>

      <main className="content">
        <ApplyDisabledBanner />
        <Outlet />
      </main>
    </div>
  );
}
