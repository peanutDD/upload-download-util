import {
  CheckCircle2,
  FolderSync,
  KeyRound,
  Link2,
  MonitorSmartphone,
} from "lucide-react";
import SettingsCard from "./SettingsCard";
import { settingsPanelClass } from "./settingsUi";

function webdavUrl() {
  if (typeof window === "undefined") {
    return "https://your-domain.example/dav"; // hardcoding-allow: SSR-only placeholder shown until browser origin is available
  }
  return `${window.location.origin}/dav`;
}

const setupSteps = [
  "Create or reuse an API Token with WebDAV enabled.",
  "Open your WebDAV client and enter the server URL.",
  "Sign in with account email plus the API Token.",
];

const clientNotes = [
  ["Finder", "Go to Server, then enter the WebDAV URL."],
  ["rclone", "Use WebDAV, vendor Other, Basic Auth."],
  ["iOS Files", "Connect to Server with the same credentials."],
  ["Note apps", "Use the shared URL for Obsidian, Joplin, or PicGo."],
];

export default function WebDavAccessSection() {
  const url = webdavUrl();

  return (
    <SettingsCard
      title="WebDAV Access"
      description="Use one endpoint and an API Token for Finder, rclone, iOS Files, and note apps."
      icon={
        <FolderSync
          className="h-[clamp(1rem,2.25vw,1.25rem)] w-[clamp(1rem,2.25vw,1.25rem)]"
          aria-hidden="true"
        />
      }
    >
      <div
        data-testid="webdav-access-grid"
        className="grid items-start gap-[clamp(1rem,2.25vw,1.25rem)]"
      >
        <section
          data-testid="webdav-connection-panel"
          className={settingsPanelClass(
            "overflow-hidden bg-[var(--settings-kpi-bg)] p-[clamp(1rem,2.25vw,1.25rem)]",
          )}
        >
          <div className="flex flex-col gap-[clamp(1rem,2.25vw,1.25rem)] sm:flex-row sm:items-start sm:justify-between">
            <div className="min-w-0">
              <div className="flex items-center gap-[clamp(0.585rem,1.35vw,0.75rem)]">
                <Link2
                  className="h-[clamp(0.9rem,2vw,1.1rem)] w-[clamp(0.9rem,2vw,1.1rem)] shrink-0 text-[var(--settings-chip-icon)]"
                  aria-hidden="true"
                />
                <h3 className="font-brand text-[length:var(--settings-text-sm)] font-semibold tracking-wide text-[var(--settings-title)]">
                  Connection details
                </h3>
              </div>
              <p className="font-brand mt-[clamp(0.39rem,0.9vw,0.5rem)] max-w-[42rem] text-[length:var(--settings-text-sm)] leading-relaxed tracking-wide text-[var(--settings-subtitle)]">
                One WebDAV address works across desktop clients, mobile file
                apps, and sync tools.
              </p>
            </div>
            <span className="font-brand inline-flex w-fit items-center rounded-full border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] px-[clamp(0.585rem,1.35vw,0.75rem)] py-[clamp(0.2925rem,0.7vw,0.375rem)] text-[length:var(--settings-text-xs)] font-semibold tracking-wide text-[var(--settings-chip-text)]">
              Basic Auth
            </span>
          </div>

          <div className="mt-[clamp(1rem,2.25vw,1.25rem)] rounded-[clamp(0.7rem,1.6vw,0.875rem)] border border-[var(--settings-panel-border)] bg-[var(--settings-form-input-bg)] p-[clamp(0.78rem,1.8vw,1rem)]">
            <p className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-panel-label)]">
              Server URL
            </p>
            <p className="mt-[clamp(0.39rem,0.9vw,0.5rem)] break-all font-mono text-[length:var(--settings-text-md)] font-semibold text-[var(--settings-panel-value)]">
              {url}
            </p>
          </div>

          <div className="mt-[clamp(0.78rem,1.8vw,1rem)] grid gap-[clamp(0.585rem,1.35vw,0.75rem)] sm:grid-cols-2">
            <div className="rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-panel-bg)] p-[clamp(0.78rem,1.8vw,1rem)]">
              <p className="font-brand text-[length:var(--settings-text-xs)] tracking-wide text-[var(--settings-kpi-label)]">
                Username
              </p>
              <p className="mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)]">
                Account email
              </p>
            </div>
            <div className="rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-panel-bg)] p-[clamp(0.78rem,1.8vw,1rem)]">
              <p className="font-brand text-[length:var(--settings-text-xs)] tracking-wide text-[var(--settings-kpi-label)]">
                Password
              </p>
              <p className="mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)]">
                API Token
              </p>
            </div>
          </div>
        </section>

        <div
          data-testid="webdav-guidance-grid"
          className="grid gap-[clamp(0.78rem,1.8vw,1rem)] lg:grid-cols-3"
        >
          <section
            data-testid="webdav-credentials-panel"
            className={settingsPanelClass("h-full")}
          >
            <div className="flex items-center gap-[clamp(0.585rem,1.35vw,0.75rem)]">
              <KeyRound
                className="h-[clamp(0.9rem,2vw,1.1rem)] w-[clamp(0.9rem,2vw,1.1rem)] shrink-0 text-[var(--settings-chip-icon)]"
                aria-hidden="true"
              />
              <h3 className="font-brand text-[length:var(--settings-text-sm)] font-semibold tracking-wide text-[var(--settings-title)]">
                Credential mapping
              </h3>
            </div>
            <p className="font-brand mt-[clamp(0.78rem,1.8vw,1rem)] text-[length:var(--settings-text-xs)] leading-relaxed tracking-wide text-[var(--settings-subtitle)]">
              Password maps to an API Token with WebDAV access enabled; account
              passwords are not accepted here.
            </p>
          </section>

          <section
            data-testid="webdav-setup-panel"
            className={settingsPanelClass("h-full")}
          >
            <div className="flex items-center gap-[clamp(0.585rem,1.35vw,0.75rem)]">
              <CheckCircle2
                className="h-[clamp(0.9rem,2vw,1.1rem)] w-[clamp(0.9rem,2vw,1.1rem)] shrink-0 text-[var(--settings-chip-icon)]"
                aria-hidden="true"
              />
              <h3 className="font-brand text-[length:var(--settings-text-sm)] font-semibold tracking-wide text-[var(--settings-title)]">
                Setup order
              </h3>
            </div>
            <ol className="mt-[clamp(0.78rem,1.8vw,1rem)] space-y-[clamp(0.585rem,1.35vw,0.75rem)]">
              {setupSteps.map((step, index) => (
                <li
                  key={step}
                  className="grid grid-cols-[clamp(1.5rem,3vw,1.75rem)_minmax(0,1fr)] gap-[clamp(0.585rem,1.35vw,0.75rem)] text-[length:var(--settings-text-sm)] leading-relaxed text-[var(--settings-panel-value)]"
                >
                  <span className="font-brand flex h-[clamp(1.5rem,3vw,1.75rem)] w-[clamp(1.5rem,3vw,1.75rem)] items-center justify-center rounded-full border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] text-[length:var(--settings-text-xs)] font-semibold text-[var(--settings-chip-text)]">
                    {index + 1}
                  </span>
                  <span>{step}</span>
                </li>
              ))}
            </ol>
          </section>

          <section
            data-testid="webdav-clients-panel"
            className={settingsPanelClass("h-full")}
          >
            <div className="flex items-center gap-[clamp(0.585rem,1.35vw,0.75rem)]">
              <MonitorSmartphone
                className="h-[clamp(0.9rem,2vw,1.1rem)] w-[clamp(0.9rem,2vw,1.1rem)] shrink-0 text-[var(--settings-chip-icon)]"
                aria-hidden="true"
              />
              <h3 className="font-brand text-[length:var(--settings-text-sm)] font-semibold tracking-wide text-[var(--settings-title)]">
                Client notes
              </h3>
            </div>
            <div className="mt-[clamp(0.78rem,1.8vw,1rem)] grid gap-[clamp(0.39rem,0.9vw,0.5rem)] sm:grid-cols-2 lg:grid-cols-1">
              {clientNotes.map(([name, detail]) => (
                <div
                  key={name}
                  className="grid gap-[clamp(0.195rem,0.45vw,0.25rem)] rounded-[clamp(0.5rem,1.2vw,0.625rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-[clamp(0.585rem,1.35vw,0.75rem)]"
                >
                  <p className="font-brand text-[length:var(--settings-text-xs)] font-semibold tracking-wide text-[var(--settings-kpi-label)]">
                    {name}
                  </p>
                  <p className="text-[length:var(--settings-text-xs)] leading-relaxed text-[var(--settings-subtitle)]">
                    {detail}
                  </p>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </SettingsCard>
  );
}
