import { useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "../store/authStore";
import { formatBytes } from "../utils/format";
import PageLayout from "../components/layout/PageLayout";
import UserInfoSection from "../components/settings/UserInfoSection";
import StorageUsageSection from "../components/settings/StorageUsageSection";
import ThemeSection from "../components/settings/ThemeSection";
import PasswordChangeSection from "../components/settings/PasswordChangeSection";
import ApiTokenSection from "../components/settings/ApiTokenSection";
import { Settings2, ArrowLeft } from "lucide-react";
import { useStorageUsage } from "../hooks/useStorageUsage";
import { useApiTokens } from "../hooks/useApiTokens";

const SETTINGS_SECTIONS = [
  { href: "#profile", label: "Account" },
  { href: "#storage", label: "Storage" },
  { href: "#appearance", label: "Appearance" },
  { href: "#security", label: "Security" },
  { href: "#api-tokens", label: "Tokens" },
];

export default function Settings() {
  const navigate = useNavigate();
  const user = useAuthStore((s) => s.user);
  const clearAuth = useAuthStore((s) => s.clearAuth);
  
  const { data: storageUsage } = useStorageUsage();
  const { data: apiTokens = [] } = useApiTokens();

  const handleLogout = useCallback(() => {
    clearAuth();
    navigate("/login");
  }, [clearAuth, navigate]);

  const handleBack = useCallback(() => {
    navigate(-1);
  }, [navigate]);

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
        className="mx-auto max-w-7xl text-[length:var(--settings-text-md)]"
        data-oid="ke2.spo"
      >
        {/* Page header (match Home neon/glass style) */}
        <div
          className="relative mb-6 overflow-hidden rounded-2xl border border-[var(--settings-surface-border)] bg-[var(--settings-surface-bg)] p-5 shadow-[var(--settings-surface-shadow)] backdrop-blur-md sm:p-6"
          data-oid="8u-ne0x"
        >
          <div
            className="pointer-events-none absolute inset-0 bg-[image:var(--settings-surface-glow)]"
            data-oid="u7wgxbr"
          />

          <div
            className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-[var(--settings-surface-hairline)] to-transparent"
            data-oid="ej.4-er"
          />

          <div
            className="relative z-10 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between"
            data-oid="5hj.--8"
          >
            <div className="min-w-0" data-oid="tc3yanx">
              <button
                type="button"
                onClick={handleBack}
                className="font-brand mb-4 inline-flex items-center rounded-xl border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] px-3 py-2 text-[length:var(--settings-text-xs)] font-semibold tracking-wide text-[var(--settings-chip-text)] hover:bg-[var(--settings-chip-bg-hover)] hover:border-[var(--settings-chip-border-hover)]"
                data-oid="li-ft82"
              >
                <ArrowLeft
                  className="mr-2 h-4 w-4 shrink-0 text-[var(--settings-chip-icon)]"
                  aria-hidden="true"
                  data-oid="mf5bp9k"
                />
                Back
              </button>
              <div className="flex items-center gap-3" data-oid="lvlcydi">
                <div
                  className="rounded-xl border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] p-2 text-[var(--settings-chip-icon)]"
                  data-oid="06je80s"
                >
                  <Settings2
                    className="h-5 w-5"
                    aria-hidden="true"
                    data-oid="6q_g5bq"
                  />
                </div>
                <h1
                  className="font-brand truncate text-[length:var(--settings-text-xl)] font-normal tracking-widest text-[var(--settings-title)]"
                  data-oid=":ph..cd"
                >
                  Settings Center
                </h1>
              </div>
              <p
                className="font-brand mt-2 text-[length:var(--settings-text-sm)] font-normal tracking-wide text-[var(--settings-subtitle)]"
                data-oid="59c-6-q"
              >
                Account info, storage quota, security & token management.
              </p>
            </div>

            <div
              className="grid grid-cols-2 gap-3 sm:grid-cols-3"
              data-oid="duty7hg"
            >
              <div
                className="rounded-xl border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-3"
                data-oid="u.f.93l"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]"
                  data-oid="a3v:fob"
                >
                  Files
                </p>
                <p
                  className="mt-1 text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] tabular-nums"
                  data-oid="xi:vm2r"
                >
                  {storageUsage ? storageUsage.file_count : "-"}
                </p>
              </div>
              <div
                className="rounded-xl border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-3"
                data-oid="oj887ag"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]"
                  data-oid="i.vmzng"
                >
                  Usage
                </p>
                <p
                  className="mt-1 text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] tabular-nums"
                  data-oid="0mw84c0"
                >
                  {storageUsage ? formatBytes(storageUsage.total_size) : "-"}
                </p>
              </div>
              <div
                className="hidden sm:block rounded-xl border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-3"
                data-oid="66o_98f"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]"
                  data-oid="75dwf9p"
                >
                  Tokens
                </p>
                <p
                  className="mt-1 text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] tabular-nums"
                  data-oid="wij1zkh"
                >
                  {apiTokens.length}
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Two-column layout: quick nav on the left, content on the right */}
        <div
          className="grid grid-cols-1 gap-6 lg:grid-cols-12"
          data-oid="xe7ci2f"
        >
          <aside className="lg:col-span-4" data-oid="j2k.gno">
            <div className="lg:sticky lg:top-28 space-y-4" data-oid="6q1rkzk">
              <div
                className="rounded-2xl border border-[var(--settings-surface-border)] bg-[var(--settings-quicknav-bg)] p-4 text-[length:var(--settings-text-sm)] text-[var(--settings-quicknav-text)] shadow-[var(--settings-quicknav-shadow)] backdrop-blur-md"
                data-oid=":40--9t"
              >
                <p
                  className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-quicknav-muted)]"
                  data-oid="9tppe.2"
                >
                  Quick nav
                </p>
                <div
                  className="mt-3 grid grid-cols-2 gap-2 sm:flex sm:flex-wrap"
                  data-oid="ofccbv_"
                >
                  {SETTINGS_SECTIONS.map((section) => (
                    <a
                      key={section.href}
                      href={section.href}
                      aria-label={`Jump to ${section.label}`}
                      className="font-brand inline-flex min-h-10 items-center justify-center rounded-xl border border-[var(--settings-chip-border)] bg-[var(--settings-chip-bg)] px-3 py-2 text-center text-[length:var(--settings-text-xs)] font-semibold tracking-wide text-[var(--settings-chip-text)] hover:border-[var(--settings-chip-border-hover)] hover:bg-[var(--settings-chip-bg-hover)] focus:outline-none focus:ring-2 focus:ring-[var(--settings-form-input-ring)]"
                    >
                      {section.label}
                    </a>
                  ))}
                </div>
              </div>
            </div>
          </aside>

          <div className="lg:col-span-8 space-y-6" data-oid="g884a9r">
            <UserInfoSection data-oid="1jpbmmd" />
            <StorageUsageSection data-oid="jcf9:wl" />
            <ThemeSection data-oid="-93dk47" />
            <PasswordChangeSection data-oid="0py-1mt" />
            <ApiTokenSection data-oid="y70j20v" />
          </div>
        </div>
      </div>
    </PageLayout>
  );
}
