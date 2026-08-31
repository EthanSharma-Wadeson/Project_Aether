import { FormEvent, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "../api/client";
import { useAuthStore } from "../store/auth";

export function LoginPage() {
  const navigate = useNavigate();
  const setAuth = useAuthStore((s) => s.setAuth);
  const clear = useAuthStore((s) => s.clear);
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("admin");
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    // Drop any stale token before login so Authorization is not attached.
    clear();
    try {
      const result = await api.login(username, password);
      setAuth(result.access_token, result.role, result.csrf_token);
      navigate("/dashboard");
    } catch (err) {
      setError(err instanceof Error ? err.message : "login failed");
    }
  }

  return (
    <div className="login-page">
      <form className="login-form card" onSubmit={onSubmit}>
        <h2>Aether</h2>
        <p className="muted">Enterprise Governance Console · operator sign-in</p>
        <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Username" />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Password"
        />
        {error && <p className="muted">{error}</p>}
        <button type="submit">Sign in</button>
      </form>
    </div>
  );
}
