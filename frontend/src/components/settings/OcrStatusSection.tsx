import { ScanText } from "lucide-react";
import { useOcrStatus } from "../../hooks/useOcrStatus";
import SettingsCard from "./SettingsCard";

function readinessLabel(available: boolean) {
  return available ? "Ready" : "Missing";
}

function readinessClass(available: boolean) {
  return available
    ? "text-[var(--settings-kpi-value)]"
    : "text-[var(--settings-danger-icon-text)]";
}

export default function OcrStatusSection() {
  const { data, isLoading } = useOcrStatus();

  const enabledLabel = data?.enabled ? "Enabled" : "Disabled";

  return (
    <SettingsCard
      title="OCR Status"
      description="Runtime readiness for image and scanned PDF content extraction."
      icon={
        <ScanText
          className="h-[clamp(1rem,2.25vw,1.25rem)] w-[clamp(1rem,2.25vw,1.25rem)]"
          aria-hidden="true"
        />
      }
    >
      <div className="grid gap-[clamp(0.78rem,1.8vw,1rem)] md:grid-cols-4">
        <StatusTile label="OCR" value={isLoading ? "Checking" : enabledLabel} />
        <StatusTile
          label="Tesseract"
          value={isLoading ? "Checking" : readinessLabel(Boolean(data?.tesseract.available))}
          valueClassName={readinessClass(Boolean(data?.tesseract.available))}
          detail={data?.tesseract.bin}
        />
        <StatusTile
          label="Poppler"
          value={isLoading ? "Checking" : readinessLabel(Boolean(data?.poppler.available))}
          valueClassName={readinessClass(Boolean(data?.poppler.available))}
          detail={data?.poppler.bin}
        />
        <StatusTile
          label="PDF OCR Limit"
          value={isLoading ? "Checking" : `${data?.pdf_max_pages ?? 0} pages`}
        />
      </div>
    </SettingsCard>
  );
}

function StatusTile({
  label,
  value,
  detail,
  valueClassName,
}: {
  label: string;
  value: string;
  detail?: string;
  valueClassName?: string;
}) {
  return (
    <div className="rounded-[clamp(0.6rem,1.4vw,0.75rem)] border border-[var(--settings-kpi-border)] bg-[var(--settings-kpi-bg)] p-[clamp(0.78rem,1.8vw,1rem)]">
      <p className="font-brand text-[length:var(--settings-text-xs)] font-normal tracking-wide text-[var(--settings-kpi-label)]">
        {label}
      </p>
      <p
        className={`mt-[clamp(0.39rem,0.9vw,0.5rem)] text-[length:var(--settings-text-sm)] font-semibold text-[var(--settings-kpi-value)] ${valueClassName ?? ""}`}
      >
        {value}
      </p>
      {detail && (
        <p className="mt-[clamp(0.195rem,0.45vw,0.25rem)] truncate font-mono text-[length:var(--settings-text-xs)] text-[var(--settings-subtitle)]">
          {detail}
        </p>
      )}
    </div>
  );
}
