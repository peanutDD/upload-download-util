import { StrictMode, startTransition } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import "./store/themeStore";
import App from "./App.tsx";
import { setupGlobalErrorTracking } from "./bootstrap/errorTracking";
import { setupApiPreconnect } from "./bootstrap/preconnect";
import { initSentry } from "./bootstrap/sentry";

initSentry();
setupApiPreconnect();
setupGlobalErrorTracking();

// PWA 已暂时禁用，待解决 EISDIR 问题后重新启用
// if (import.meta.env.PROD) {
//   import('virtual:pwa-register').then(({ registerSW }) => {
//     registerSW({ immediate: true });
//   });
// }

const root = createRoot(document.getElementById("root")!);

startTransition(() => {
  root.render(
    <StrictMode data-oid="e9x8hi9">
      <App data-oid="a-ygtsq" />
    </StrictMode>,
  );
});

// 可选：通过 `?vitals=1` 输出 Web Vitals（默认关闭，零侵入）
try {
  const params = new URLSearchParams(window.location.search);
  if (params.has("vitals")) {
    void import("./vitals").then((m) => m.reportVitals());
  }
} catch {
  // no-op: 非浏览器环境/极端情况下忽略
}
