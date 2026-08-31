import { create } from "zustand";

type AuthState = {
  token: string | null;
  role: string | null;
  csrfToken: string | null;
  setAuth: (token: string, role: string, csrfToken?: string) => void;
  setCsrf: (csrfToken: string) => void;
  clear: () => void;
};

/** Detect JWTs issued before iss/aud/typ were required (causes "missing field 'iss'"). */
export function accessTokenHasRequiredClaims(token: string | null): boolean {
  if (!token) return false;
  try {
    const parts = token.split(".");
    if (parts.length < 2) return false;
    const b64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
    const padded = b64 + "=".repeat((4 - (b64.length % 4)) % 4);
    const payload = JSON.parse(atob(padded)) as Record<string, unknown>;
    return (
      typeof payload.iss === "string" &&
      typeof payload.aud === "string" &&
      typeof payload.typ === "string" &&
      typeof payload.sub === "string"
    );
  } catch {
    return false;
  }
}

function readInitialAuth(): Pick<AuthState, "token" | "role" | "csrfToken"> {
  const token = localStorage.getItem("cp_token");
  const role = localStorage.getItem("cp_role");
  const csrfToken = sessionStorage.getItem("cp_csrf");
  if (token && !accessTokenHasRequiredClaims(token)) {
    localStorage.removeItem("cp_token");
    localStorage.removeItem("cp_role");
    sessionStorage.removeItem("cp_csrf");
    return { token: null, role: null, csrfToken: null };
  }
  return { token, role, csrfToken };
}

export const useAuthStore = create<AuthState>((set) => ({
  ...readInitialAuth(),
  setAuth: (token, role, csrfToken) => {
    localStorage.setItem("cp_token", token);
    localStorage.setItem("cp_role", role);
    if (csrfToken) {
      sessionStorage.setItem("cp_csrf", csrfToken);
    }
    set({ token, role, csrfToken: csrfToken ?? sessionStorage.getItem("cp_csrf") });
  },
  setCsrf: (csrfToken) => {
    sessionStorage.setItem("cp_csrf", csrfToken);
    set({ csrfToken });
  },
  clear: () => {
    localStorage.removeItem("cp_token");
    localStorage.removeItem("cp_role");
    sessionStorage.removeItem("cp_csrf");
    set({ token: null, role: null, csrfToken: null });
  },
}));
