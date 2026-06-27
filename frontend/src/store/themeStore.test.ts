import { beforeEach, describe, expect, it } from "vitest";
import { useThemeStore } from "./themeStore";

function resetThemeStore() {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.classList.remove("dark", "light", "purple", "terminal", "portfolio");
  useThemeStore.getState().setTheme("dark");
}

describe("theme store", () => {
  beforeEach(() => {
    resetThemeStore();
  });

  it("accepts and persists the Terminal theme while keeping dark mode base classes", () => {
    useThemeStore.getState().setTheme("terminal");

    expect(useThemeStore.getState()).toMatchObject({
      theme: "terminal",
      effectiveTheme: "terminal",
    });
    expect(document.documentElement).toHaveAttribute("data-theme", "terminal");
    expect(document.documentElement).toHaveClass("dark", "terminal");
    expect(document.documentElement).not.toHaveClass("light", "purple", "portfolio");

    const persistedTheme = JSON.parse(localStorage.getItem("theme-storage") ?? "{}");
    expect(persistedTheme.state.theme).toBe("terminal");
    expect(persistedTheme.state.effectiveTheme).toBe("terminal");
  });

  it("accepts and persists the Portfolio theme while keeping dark mode base classes", () => {
    useThemeStore.getState().setTheme("portfolio");

    expect(useThemeStore.getState()).toMatchObject({
      theme: "portfolio",
      effectiveTheme: "portfolio",
    });
    expect(document.documentElement).toHaveAttribute("data-theme", "portfolio");
    expect(document.documentElement).toHaveClass("dark", "portfolio");
    expect(document.documentElement).not.toHaveClass("light", "purple", "terminal");

    const persistedTheme = JSON.parse(localStorage.getItem("theme-storage") ?? "{}");
    expect(persistedTheme.state.theme).toBe("portfolio");
    expect(persistedTheme.state.effectiveTheme).toBe("portfolio");
  });

  it("cycles themes from dark to light to purple to terminal to portfolio and back to dark", () => {
    const toggleTheme = useThemeStore.getState().toggleTheme;

    toggleTheme();
    expect(useThemeStore.getState().effectiveTheme).toBe("light");

    toggleTheme();
    expect(useThemeStore.getState().effectiveTheme).toBe("purple");

    toggleTheme();
    expect(useThemeStore.getState().effectiveTheme).toBe("terminal");

    toggleTheme();
    expect(useThemeStore.getState().effectiveTheme).toBe("portfolio");

    toggleTheme();
    expect(useThemeStore.getState().effectiveTheme).toBe("dark");
  });
});
