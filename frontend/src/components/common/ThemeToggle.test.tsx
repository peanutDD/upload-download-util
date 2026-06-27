import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { useThemeStore } from "../../store/themeStore";
import ThemeToggle from "./ThemeToggle";

function resetThemeStore() {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.classList.remove("dark", "light", "purple", "terminal", "portfolio");
  useThemeStore.getState().setTheme("dark");
}

describe("ThemeToggle", () => {
  beforeEach(() => {
    resetThemeStore();
  });

  it("announces Terminal as the fourth theme and cycles it to Portfolio", async () => {
    useThemeStore.getState().setTheme("terminal");

    render(<ThemeToggle showLabel />);

    const toggle = screen.getByRole("button", {
      name: /current Terminal, click to switch to Portfolio/i,
    });

    expect(toggle).toHaveTextContent("Terminal");
    await userEvent.click(toggle);

    expect(useThemeStore.getState().effectiveTheme).toBe("portfolio");
    expect(screen.getByRole("button", { name: /current Portfolio, click to switch to Dark/i })).toBeInTheDocument();
  });

  it("announces Portfolio as the fifth theme and cycles it back to Dark", async () => {
    useThemeStore.getState().setTheme("portfolio");

    render(<ThemeToggle showLabel />);

    const toggle = screen.getByRole("button", {
      name: /current Portfolio, click to switch to Dark/i,
    });

    expect(toggle).toHaveTextContent("Portfolio");
    await userEvent.click(toggle);

    expect(useThemeStore.getState().effectiveTheme).toBe("dark");
    expect(screen.getByRole("button", { name: /current Dark, click to switch to Light/i })).toBeInTheDocument();
  });
});
