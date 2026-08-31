import { create } from "zustand";

type Theme = "dark" | "light";

type UiState = {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
};

function applyTheme(theme: Theme) {
  document.documentElement.setAttribute("data-theme", theme);
}

const stored = (localStorage.getItem("cp_theme") as Theme | null) ?? "dark";
applyTheme(stored);

export const useUiStore = create<UiState>((set, get) => ({
  theme: stored,
  setTheme: (theme) => {
    localStorage.setItem("cp_theme", theme);
    applyTheme(theme);
    set({ theme });
  },
  toggleTheme: () => {
    const next = get().theme === "dark" ? "light" : "dark";
    get().setTheme(next);
  },
}));
