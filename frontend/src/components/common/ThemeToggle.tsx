//! # Theme Toggle Component
//!
//! 主题切换按钮组件，支持深色/浅色模式切换。

import { Braces, Moon, Sparkles, Sun, Terminal } from "lucide-react";
import { useThemeStore } from "../../store/themeStore";
import type { Theme } from "../../store/themeStore";
import { cn } from "../../utils/cn";

interface ThemeToggleProps {
  className?: string;
  showLabel?: boolean;
}

/**
 * 主题切换按钮
 *
 * 点击可在深色和浅色模式之间切换。
 */
export default function ThemeToggle({
  className,
  showLabel = false,
}: ThemeToggleProps) {
  const { effectiveTheme, toggleTheme } = useThemeStore();
  const themeLabels: Record<Theme, string> = {
    dark: "Dark",
    light: "Light",
    purple: "Purple",
    terminal: "Terminal",
    portfolio: "Portfolio",
  };
  const nextThemeByCurrent: Record<Theme, Theme> = {
    dark: "light",
    light: "purple",
    purple: "terminal",
    terminal: "portfolio",
    portfolio: "dark",
  };
  const currentThemeLabel = themeLabels[effectiveTheme];
  const nextThemeLabel = themeLabels[nextThemeByCurrent[effectiveTheme]];

  return (
    <button
      type="button"
      onClick={toggleTheme}
      className={cn(
        "nav-btn inline-flex items-center justify-center whitespace-nowrap",
        "nav-ui-fluid font-semibold tracking-wide text-[var(--nav-btn-text)]",
        "bg-[var(--nav-btn-bg)] border border-[var(--nav-btn-border)]",
        "hover:bg-[var(--nav-btn-bg-hover)] hover:border-[var(--nav-btn-border-hover)]",
        "active:translate-y-px transition-all duration-200",
        className,
      )}
      aria-label={`Switch theme: current ${currentThemeLabel}, click to switch to ${nextThemeLabel}`}
      title={`Current: ${currentThemeLabel} (Click to switch)`}
      data-oid="6nbyj70"
    >
      {effectiveTheme === "portfolio" ? (
        <Braces
          className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
          aria-hidden="true"
          data-oid="8hvxm0e"
        />
      ) : effectiveTheme === "terminal" ? (
        <Terminal
          className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
          aria-hidden="true"
          data-oid="9h6tnl2"
        />
      ) : effectiveTheme === "light" ? (
        <Sun
          className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
          aria-hidden="true"
          data-oid="zeovdrg"
        />
      ) : effectiveTheme === "dark" ? (
        <Moon
          className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
          aria-hidden="true"
          data-oid="m1.w5ph"
        />
      ) : (
        <Sparkles
          className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
          aria-hidden="true"
          data-oid="yk8.6nq"
        />
      )}
      {showLabel && (
        <span className="hidden sm:inline whitespace-nowrap" data-oid="6rqexbj">
          {currentThemeLabel}
        </span>
      )}
    </button>
  );
}
