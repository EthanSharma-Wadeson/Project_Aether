import { Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "./components/Layout";
import { accessTokenHasRequiredClaims, useAuthStore } from "./store/auth";
import { AdminPage } from "./pages/AdminPage";
import { AgentDetailPage } from "./pages/AgentDetailPage";
import { AgentsPage } from "./pages/AgentsPage";
import { ApplyPage } from "./pages/ApplyPage";
import { AuditPage } from "./pages/AuditPage";
import { CapabilitiesPage } from "./pages/CapabilitiesPage";
import { DashboardPage } from "./pages/DashboardPage";
import { EconomicPage } from "./pages/EconomicPage";
import { LoginPage } from "./pages/LoginPage";
import { PoliciesPage } from "./pages/PoliciesPage";
import { PolicyDetailPage } from "./pages/PolicyDetailPage";
import { ReconcilePage } from "./pages/ReconcilePage";
import { ReputationPage } from "./pages/ReputationPage";
import { SecurityCentrePage } from "./pages/SecurityCentrePage";
import { SettlementsPage } from "./pages/SettlementsPage";
import { TreasuryRoutes } from "./pages/TreasuryPages";

function Protected({ children }: { children: JSX.Element }) {
  const token = useAuthStore((s) => s.token);
  const clear = useAuthStore((s) => s.clear);
  if (!token) return <Navigate to="/login" replace />;
  // Extra guard if store was hydrated before claim check ran in another tab.
  if (!accessTokenHasRequiredClaims(token)) {
    clear();
    return <Navigate to="/login" replace />;
  }
  return children;
}

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route
        path="/"
        element={
          <Protected>
            <Layout />
          </Protected>
        }
      >
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<DashboardPage />} />
        <Route path="agents" element={<AgentsPage />} />
        <Route path="agents/:id" element={<AgentDetailPage />} />
        <Route path="capabilities" element={<CapabilitiesPage />} />
        <Route path="economic" element={<EconomicPage />} />
        <Route path="settlements" element={<SettlementsPage />} />
        <Route path="reputation" element={<ReputationPage />} />
        <Route path="policies" element={<PoliciesPage />} />
        <Route path="policies/:id" element={<PolicyDetailPage />} />
        <Route path="apply" element={<ApplyPage />} />
        <Route path="treasury/*" element={<TreasuryRoutes />} />
        <Route path="audit" element={<AuditPage />} />
        <Route path="security" element={<SecurityCentrePage />} />
        <Route path="reconcile" element={<ReconcilePage />} />
        <Route path="admin" element={<AdminPage />} />
      </Route>
    </Routes>
  );
}
