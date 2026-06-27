import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import OcrStatusSection from "./OcrStatusSection";
import { useOcrStatus } from "../../hooks/useOcrStatus";

vi.mock("../../hooks/useOcrStatus", () => ({
  useOcrStatus: vi.fn(),
}));

describe("OcrStatusSection", () => {
  it("shows OCR runtime readiness and PDF page limit", () => {
    vi.mocked(useOcrStatus).mockReturnValue({
      data: {
        enabled: true,
        pdf_max_pages: 8,
        tesseract: {
          bin: "tesseract",
          available: true,
        },
        poppler: {
          bin: "pdftoppm",
          available: false,
        },
      },
      isLoading: false,
    } as unknown as ReturnType<typeof useOcrStatus>);

    render(<OcrStatusSection />);

    expect(screen.getByText("OCR Status")).toBeInTheDocument();
    expect(screen.getByText("Enabled")).toBeInTheDocument();
    expect(screen.getByText("Tesseract")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(screen.getByText("Poppler")).toBeInTheDocument();
    expect(screen.getByText("Missing")).toBeInTheDocument();
    expect(screen.getByText("8 pages")).toBeInTheDocument();
  });
});
