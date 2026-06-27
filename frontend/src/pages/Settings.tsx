import { useCallback } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useAuthStore } from "../store/authStore";
import { formatBytes } from "../utils/format";
import PageLayout from "../components/layout/PageLayout";
import UserInfoSection from "../components/settings/UserInfoSection";
import StorageUsageSection from "../components/settings/StorageUsageSection";
import PasswordChangeSection from "../components/settings/PasswordChangeSection";
import ApiTokenSection from "../components/settings/ApiTokenSection";
import WebDavAccessSection from "../components/settings/WebDavAccessSection";
import OcrStatusSection from "../components/settings/OcrStatusSection";
import { Settings2, ArrowLeft } from "lucide-react";
import { useStorageUsage } from "../hooks/useStorageUsage";
import { useApiTokens } from "../hooks/useApiTokens";

function hasPreviousAppHistoryEntry(locationKey: string) {
  if (typeof window !== "undefined") {
    const historyIndex = window.history.state?.idx;
    if (typeof historyIndex === "number") {
      return historyIndex > 0;
    }
  }
  return locationKey !== "default";
}

export default function Settings() {
  const navigate = useNavigate();
  const location = useLocation();
  const user = useAuthStore((s) => s.user);
  const clearAuth = useAuthStore((s) => s.clearAuth);
  
  const { data: storageUsage } = useStorageUsage();
  const { data: apiTokens = [] } = useApiTokens();

  const handleLogout = useCallback(() => {
    clearAuth();
    navigate("/login");
  }, [clearAuth, navigate]);

  const handleBack = useCallback(() => {
    if (hasPreviousAppHistoryEntry(location.key)) {
      navigate(-1);
    } else {
      navigate("/files", { replace: true });
    }
  }, [location.key, navigate]);

  return (
    <PageLayout
      title="SETTINGS"
      username={user?.username}
      onLogout={handleLogout}
      showSettings={false}
      data-oid="nr697ev"
    >
      {/* Match NavBar width so the logo aligns with page content */}
      <div
        className="mx-auto max-w-[80rem] text-[length:var(--settings-text-md)]"
        data-oid="ke2.spo"
      >
        {/* Page header (match Home neon/glass style) */}
        <div
          data-testid="settings-hero-panel"
          className="relative isolate mb-[clamp(1.25rem,2.7vw,1.5rem)] overflow-hidden rounded-[clamp(0.8rem,2vw,1rem)] border border-[var(--settings-surface-border)] bg-[var(--settings-surface-bg)] p-[clamp(1rem,2.25vw,1.25rem)] shadow-[var(--settings-surface-shadow)] backdrop-blur-md transition-[border-color,box-shadow] duration-300 sm:p-[clamp(1.25rem,2.7vw,1.5rem)]"
          data-oid="8u-ne0x"
        >
          <div
            className="pointer-events-none absolute inset-0 bg-[image:var(--settings-surface-glow)]"
            data-oid="u7wgxbr"
          />

          <div className="pointer-events-none absolute inset-x-[clamp(1rem,2.25vw,1.5rem)] top-0 h-px bg-gradient-to-r from-transparent via-[var(--settings-surface-hairline)] to-transparent" />

          <div className="pointer-events-none absolute bottom-0 left-[clamp(1rem,2.25vw,1.5rem)] right-[clamp(1rem,2.25vw,1.5rem)] h-px bg-gradient-to-r from-transparent via-[var(--settings-surface-hairline)] to-transparent opacity-70" />

          <div
            className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-[var(--settings-surface-hairline)] to-transparent"
            data-oid="ej.4-er"
          />

          <div
            className="relative z-10 flex flex-col gap-[clamp(0.78rem,1.8vw,1rem)] sm:flex-row sm:items-center sm:justify-between"
            data-oid="5hj.--8"
          >
            <div className="min-w-0" data-oid="tc3yanx">
              <button
                type="button"
                onClick={handleBack}
                className="font-brand mb-[clamp(0.78rem,1.8vw,1rem)] inline-flex items-center rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] px-[clamp(0.585rem,1.35vw,0.75rem)] py-[clamp(0.39rem,0.9vw,0.5rem)] text-[length:var(--settings-text-xs)] font-semibold tracking-wide text-[var(--settings-chip-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.05)] transition-[border-color,background-color,transform] duration-200 hover:-translate-y-px hover:bg-[var(--settings-chip-bg-hover)] hover:border-[var(--settings-chip-border-hover)] active:translate-y-0"
                data-oid="li-ft82"
              >
                <ArrowLeft
                  className="mr-[clamp(0.39rem,0.9vw,0.5rem)] h-[clamp(0.78rem,1.8vw,1rem)] w-[clamp(0.78rem,1.8vw,1rem)] shrink-0 text-[var(--settings-chip-icon)]"
                  aria-hidden="true"
                  data-oid="mf5bp9k"
                />
                Back
              </button>
              <div className="flex items-center gap-[clamp(0.585rem,1.35vw,0.75rem)]" data-oid="lvlcydi">
                <button
                  type="button"
                  aria-label="Go to files home"
                  onClick={() => navigate("/files")}
                  className="rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] p-[clamp(0.39rem,0.9vw,0.5rem)] text-[var(--settings-chip-icon)] shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_0.75rem_1.75rem_rgba(16,185,129,0.08)] transition-[border-color,background-color,transform] duration-200 hover:-translate-y-px hover:border-[var(--settings-chip-border-hover)] hover:bg-[var(--settings-chip-bg-hover)] focus:outline-none focus:ring-2 focus:ring-[var(--settings-form-input-ring)] active:translate-y-0"
                  data-oid="06je80s"
                >
                  <Settings2
                    className="h-[clamp(1rem,2.25vw,1.25rem)] w-[clamp(1rem,2.25vw,1.25rem)]"
                    aria-hidden="true"
                    data-oid="6q_g5bq"
                  />
                </button>
                <h1
                  className="font-brand truncate text-[length:var(--settings-text-xl)] font-normal tracking-widest text-[var(--settings-title)]"
                  data-oid=":ph..cd"
                >
                  Settings Center
                </h1>
              </div>
              <p
                className="font-brand mt-[clamp(0.39rem,0.9vw,0.5rem)] text-[length:var(--settings-text-sm)] font-normal tracking-wide text-[var(--settings-subtitle)]"
                data-oid="59c-6-q"
              >
                Account info, storage quota, security & token management.
              </p>
            </div>

            <div
              className="grid grid-cols-2 gap-[clamp(0.585rem,1.35vw,0.75rem)] sm:grid-cols-3"
              data-oid="duty7hg"
            >
              <div
                className="rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-[clamp(0.585rem,1.35vw,0.75rem)] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
                data-oid="u.f.93l"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]"
                  data-oid="a3v:fob"
                >
                  Files
                </p>
                <p
                  className="mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] tabular-nums"
                  data-oid="xi:vm2r"
                >
                  {storageUsage ? storageUsage.file_count : "-"}
                </p>
              </div>
              <div
                className="rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-[clamp(0.585rem,1.35vw,0.75rem)] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
                data-oid="oj887ag"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]"
                  data-oid="i.vmzng"
                >
                  Usage
                </p>
                <p
                  className="mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] tabular-nums"
                  data-oid="0mw84c0"
                >
                  {storageUsage ? formatBytes(storageUsage.total_size) : "-"}
                </p>
              </div>
              <div
                className="hidden rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-[clamp(0.585rem,1.35vw,0.75rem)] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] sm:block"
                data-oid="66o_98f"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]"
                  data-oid="75dwf9p"
                >
                  Tokens
                </p>
                <p
                  className="mt-[clamp(0.195rem,0.45vw,0.25rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] tabular-nums"
                  data-oid="wij1zkh"
                >
                  {apiTokens.length}
                </p>
              </div>
            </div>
          </div>
        </div>

        <div
          data-testid="settings-card-grid"
          className="grid gap-[clamp(1.35rem,3vw,1.75rem)]"
          data-oid="g884a9r"
        >
          <div
            data-testid="settings-group-identity"
            className="grid gap-[clamp(1.25rem,2.7vw,1.5rem)] xl:grid-cols-2 xl:items-stretch"
          >
            <div
              data-testid="settings-account-column"
              className="min-w-0 h-full [&>section]:h-full [&>section]:flex [&>section]:flex-col [&>section>div:last-child]:flex [&>section>div:last-child]:flex-1 [&>section>div:last-child>form]:flex [&>section>div:last-child>form]:flex-1 [&>section>div:last-child>form]:flex-col"
            >
              <UserInfoSection data-oid="1jpbmmd" />
            </div>
            <div
              data-testid="settings-security-column"
              className="min-w-0 h-full [&>section]:h-full [&>section]:flex [&>section]:flex-col [&>section>div:last-child]:flex [&>section>div:last-child]:flex-1 [&>section>div:last-child>form]:flex [&>section>div:last-child>form]:flex-1 [&>section>div:last-child>form]:flex-col"
            >
              <PasswordChangeSection data-oid="0py-1mt" />
            </div>
          </div>

          <div
            data-testid="settings-group-webdav-focus"
            className="min-w-0 xl:[&>section]:p-[clamp(1.35rem,2.8vw,1.75rem)]"
          >
            <WebDavAccessSection />
          </div>

          <div
            data-testid="settings-group-token-workspace"
            className="min-w-0 xl:[&>section]:p-[clamp(1.35rem,2.8vw,1.75rem)]"
          >
            <ApiTokenSection data-oid="y70j20v" />
          </div>

          <div
            data-testid="settings-group-status"
            className="grid gap-[clamp(1.25rem,2.7vw,1.5rem)] lg:grid-cols-2 lg:items-stretch"
          >
            <div
              data-testid="settings-ocr-column"
              className="min-w-0 h-full [&>section]:h-full"
            >
              <OcrStatusSection />
            </div>
            <div
              data-testid="settings-storage-column"
              className="min-w-0 h-full [&>section]:h-full"
            >
              <StorageUsageSection data-oid="jcf9:wl" />
            </div>
          </div>
        </div>
      </div>
    </PageLayout>
  );
}
