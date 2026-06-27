import { useEffect, useState } from "react";

const DESKTOP_DRAG_POINTER_QUERY = "(hover: hover) and (pointer: fine)";

function canUseNativeDrag() {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return true;
  }
  return window.matchMedia(DESKTOP_DRAG_POINTER_QUERY).matches;
}

export function useNativeDragEnabled() {
  const [enabled, setEnabled] = useState(canUseNativeDrag);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return;
    }

    const query = window.matchMedia(DESKTOP_DRAG_POINTER_QUERY);
    const update = () => setEnabled(query.matches);
    update();
    query.addEventListener?.("change", update);
    query.addListener?.(update);
    return () => {
      query.removeEventListener?.("change", update);
      query.removeListener?.(update);
    };
  }, []);

  return enabled;
}
