import type { ReactNode } from "react";
import { cn } from "../../utils/cn";

interface SettingsCardProps {
  id?: string;
  title: string;
  description?: string;
  icon?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}

export default function SettingsCard({
  id,
  title,
  description,
  icon,
  actions,
  children,
  className,
}: SettingsCardProps) {
  return (
    <section
      id={id}
      className={cn(
        // Reserve space for the fixed NavBar when using anchor links.
        "scroll-mt-[clamp(6.75rem,12.6vw,7rem)]",
        "group/settings-card relative isolate overflow-hidden rounded-[clamp(0.8rem,2vw,1rem)]",
        "border border-[var(--settings-surface-border)] bg-[var(--settings-surface-bg)]",
        "shadow-[var(--settings-surface-shadow)]",
        "backdrop-blur-md transition-[border-color,box-shadow,transform] duration-300",
        "hover:border-[var(--settings-panel-border-hover)] hover:shadow-[var(--settings-surface-shadow)]",
        "text-[length:var(--settings-text-md)]",
        "p-[clamp(1rem,2.25vw,1.25rem)] sm:p-[clamp(1.25rem,2.7vw,1.5rem)]",
        className,
      )}
      data-testid="settings-card-shell"
      data-oid="au1p9wx"
    >
      {/* Ambient glow + top hairline (match the Home / Nav style). */}
      <div
        className="pointer-events-none absolute inset-0 bg-[image:var(--settings-surface-glow)]"
        data-oid="ryeionw"
      />

      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-[var(--settings-surface-hairline)] to-transparent"
        data-oid="ypzl8tm"
      />

      <div className="pointer-events-none absolute inset-x-[clamp(1rem,2.25vw,1.25rem)] top-0 h-px bg-gradient-to-r from-transparent via-[var(--settings-surface-hairline)] to-transparent opacity-0 transition-opacity duration-300 group-hover/settings-card:opacity-60" />

      <div className="pointer-events-none absolute inset-y-[clamp(1rem,2.25vw,1.25rem)] left-0 w-px bg-gradient-to-b from-transparent via-[var(--settings-surface-hairline)] to-transparent opacity-55" />

      <header
        className="relative z-10 flex items-start gap-[clamp(0.585rem,1.35vw,0.75rem)]"
        data-oid="07m066k"
      >
        {icon && (
          <div
            className="mt-[clamp(0.0975rem,0.3vw,0.125rem)] shrink-0 rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] p-[clamp(0.39rem,0.9vw,0.5rem)] text-[var(--settings-chip-icon)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08),0_0.75rem_1.75rem_rgba(16,185,129,0.08)] transition-[border-color,background-color,box-shadow] duration-300 group-hover/settings-card:border-[var(--settings-chip-border-hover)] group-hover/settings-card:bg-[var(--settings-chip-bg-hover)]"
            data-oid="9ktkh.m"
          >
            {icon}
          </div>
        )}
        <div className="min-w-0 flex-1" data-oid="6rho:-6">
          <h2
            className="font-brand text-[length:var(--settings-text-lg)] font-semibold tracking-wide text-[var(--settings-title)]"
            data-oid="h.-c1fg"
          >
            {title}
          </h2>
          {description && (
            <p
              className="font-brand mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-normal leading-relaxed tracking-wide text-[var(--settings-subtitle)]"
              data-oid="vftcw.7"
            >
              {description}
            </p>
          )}
        </div>
        {actions && (
          <div className="shrink-0" data-oid="dt:zj1u">
            {actions}
          </div>
        )}
      </header>

      <div className="relative z-10 mt-[clamp(1rem,2.25vw,1.25rem)]" data-oid="4462rmf">
        {children}
      </div>
    </section>
  );
}
