import api from "./api";

export interface OcrRuntimeDependency {
  bin: string;
  available: boolean;
}

export interface OcrStatus {
  enabled: boolean;
  pdf_max_pages: number;
  tesseract: OcrRuntimeDependency;
  poppler: OcrRuntimeDependency;
}

export const ocrStatusService = {
  async getStatus(): Promise<OcrStatus> {
    const response = await api.get<OcrStatus>("/api/files/search/ocr/status");
    return response.data;
  },
};
