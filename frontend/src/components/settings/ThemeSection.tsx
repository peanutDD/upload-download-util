import { Palette } from "lucide-react";
import { cn } from "../../utils/cn";
import { useThemeStore } from "../../store/themeStore";
import type { Theme } from "../../store/themeStore";
import SettingsCard from "./SettingsCard";

const OPTIONS: Array<{
  value: Theme;
  title: string;
  description: string;
}> = [
  { value: "dark", title: "Dark", description: "Green dark theme." },
  { value: "light", title: "Light", description: "Use light theme." },
  { value: "purple", title: "Purple", description: "Classic purple theme." },
  { value: "terminal", title: "Terminal", description: "Blackglass neon terminal." },
  { value: "portfolio", title: "Portfolio", description: "Neon developer portfolio." },
];

export default function ThemeSection() {
  const { theme, effectiveTheme, setTheme } = useThemeStore();

  return (
    <SettingsCard
      id="appearance"
      title="Appearance"
      description="Choose how the interface should look."
      icon={
        <Palette className="h-[clamp(1rem,2.25vw,1.25rem)] w-[clamp(1rem,2.25vw,1.25rem)]" aria-hidden="true" data-oid="gzpmni9" />
      }
      data-oid="gg52g.s"
    >
      <div className="grid gap-[clamp(0.39rem,0.9vw,0.5rem)] sm:grid-cols-2 xl:grid-cols-5" data-oid="yfy4n_7">
        {OPTIONS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => setTheme(opt.value)}
            className={cn(
              "rounded-[clamp(0.6rem,1.4vw,0.75rem)] border px-[clamp(0.78rem,1.8vw,1rem)] py-[clamp(0.585rem,1.35vw,0.75rem)] text-left transition-colors",
              "min-h-[5.75rem]",
              "bg-[var(--settings-panel-bg)] border-[var(--settings-panel-border)] text-[var(--color-text-primary)]",
              "hover:bg-[var(--glass-bg-strong)] hover:border-[var(--color-border-medium)]",
              theme === opt.value &&
                "bg-[var(--settings-chip-bg-hover)] border-[var(--cta-primary-border)] shadow-[var(--shadow-glass-card)]",
            )}
            aria-pressed={theme === opt.value}
            data-oid="uxa2l.x"
          >
            <div
              className="font-brand text-[length:var(--settings-text-sm)] font-semibold tracking-wide"
              data-oid="wej7z3c"
            >
              {opt.title}
            </div>
            <div
              className="font-brand mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-xs)] tracking-wide text-[var(--color-text-muted)]"
              data-oid="pb91c.w"
            >
              {opt.description}
            </div>
          </button>
        ))}
      </div>

      <div
        className="mt-[clamp(0.78rem,1.8vw,1rem)] rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-panel-border)] bg-[var(--settings-panel-bg)] p-[clamp(0.78rem,1.8vw,1rem)]"
        data-oid="t8wonjh"
      >
        <div
          className="font-brand text-[length:var(--settings-text-xs)] tracking-wide text-[var(--settings-panel-label)]"
          data-oid="k60k6cl"
        >
          Current
        </div>
        <div
          className="mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-panel-value)]"
          data-oid="48opr-f"
        >
          {effectiveTheme}
        </div>
      </div>
    </SettingsCard>
  );
}
