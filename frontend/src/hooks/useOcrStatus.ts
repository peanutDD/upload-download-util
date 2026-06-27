import { useQuery } from "@tanstack/react-query";
import { ocrStatusService, type OcrStatus } from "../services/ocrStatus";

export function useOcrStatus() {
  return useQuery<OcrStatus>({
    queryKey: ["ocrStatus"],
    queryFn: () => ocrStatusService.getStatus(),
    staleTime: 1000 * 60,
  });
}
