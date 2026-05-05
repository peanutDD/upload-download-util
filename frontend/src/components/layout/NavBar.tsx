import { useNavigate } from "react-router-dom";
import { APP_NAME } from "../../config/env";
import { cn } from "../../utils/cn";
import { ArrowLeft, LogOut, Settings } from "lucide-react";
import ThemeToggle from "../common/ThemeToggle";

interface NavBarProps {
  title?: string;
  backTo?: { path: string; label: string };
  username?: string | null;
  onLogout: () => void;
  extra?: React.ReactNode;
  showSettings?: boolean;
}

export default function NavBar({
  title = APP_NAME,
  backTo,
  username,
  onLogout,
  extra,
  showSettings = true,
}: NavBarProps) {
  const navigate = useNavigate();
  const handleDoubleClick = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  return (
    <nav
      className={cn(
        "fixed inset-x-0 top-0 z-50 overflow-hidden bg-nav-surface pt-[env(safe-area-inset-top)] backdrop-blur-[var(--nav-surface-blur)]"
      )}
      onDoubleClick={handleDoubleClick}
      data-oid="2f5wd3k"
    >
      {/* 顶部紫色微光线条（对齐参考图） */}
      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-px bg-[image:var(--nav-top-glow)]"
        data-oid="uu11gmg"
      />

      {/* 底部青绿色分隔线（对齐参考图） */}
      <div
        className="pointer-events-none absolute inset-x-0 bottom-0 h-0.5 bg-[var(--nav-bottom-line)]"
        data-oid="mpflc4s"
      />

      {/* 两侧淡紫氛围光 */}
      <div
        className="pointer-events-none absolute inset-0 bg-[image:var(--nav-side-ambience)]"
        data-oid="u0bfbwi"
      />

      <div
        className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8"
        data-oid="q:y39os"
      >
        <div
          className="flex justify-between items-center h-[clamp(4.75rem,7.6vw,6.25rem)]"
          data-oid="qlp0jdz"
        >
          <div className="flex items-center gap-4 min-w-0" data-oid="trr68n.">
            {backTo && (
              <button
                type="button"
                onClick={() => navigate(backTo.path)}
                className={cn(
                  "nav-btn inline-flex items-center justify-center whitespace-nowrap",
                  "nav-ui-fluid font-semibold tracking-wide text-[var(--nav-btn-text)]",
                  "bg-[var(--nav-btn-bg)] border border-[var(--nav-btn-border)]",
                  "hover:bg-[var(--nav-btn-bg-hover)] hover:border-[var(--nav-btn-border-hover)]",
                  "active:translate-y-px transition-all duration-200",
                )}
                aria-label={backTo.label}
                title={backTo.label}
                data-oid="ao_vrim"
              >
                <ArrowLeft
                  className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
                  aria-hidden="true"
                  data-oid="-00ebq2"
                />

                <span
                  className="hidden sm:inline whitespace-nowrap"
                  data-oid="9cfh85y"
                >
                  {backTo.label}
                </span>
              </button>
            )}

            <div className="flex items-center gap-4 min-w-0" data-oid="iqbhzo8">
              <div className="shrink-0 select-none" data-oid="mtnbm56">
                <CyberPrismLogo
                  className="h-[clamp(2.6rem,4vw,3.3rem)] w-[clamp(2.6rem,4vw,3.3rem)]"
                  data-oid="b:z042i"
                />
              </div>
              <h1
                className="nav-title-fluid font-brand truncate font-normal tracking-widest text-[var(--nav-title-text)] drop-shadow-sm select-none"
                data-oid="m354-f5"
              >
                {title}
              </h1>
            </div>
          </div>

          <div
            className="flex items-center gap-1.5 sm:gap-2"
            data-oid=".t1_g6i"
          >
            {extra}

            {/* 右侧“控制面板” */}
            <div
              className={cn(
                "nav-panel relative flex items-center shrink-0",
                "bg-[var(--nav-panel-bg)] border border-[var(--nav-panel-border)]",
                "shadow-[var(--nav-panel-shadow)]",
              )}
              data-oid="grfmuct"
            >
              {/* 细微霓虹描边 */}
              <div
                className="pointer-events-none absolute inset-0 bg-[image:var(--nav-panel-edge-glow)]"
                data-oid="5nuvn0z"
              />

              {username != null && (
                <div
                  className={cn(
                    "nav-chip hidden sm:flex items-center",
                    "bg-[var(--nav-chip-bg)] border border-[var(--nav-chip-border)]",
                    "text-[var(--nav-chip-text)]",
                  )}
                  title={username}
                  data-oid="s_n1clb"
                >
                  <span
                    className="nav-dot bg-[var(--nav-user-dot-bg)] shadow-[var(--nav-user-dot-shadow)]"
                    data-oid="t.9r114"
                  />

                  <span
                    className="nav-ui-fluid font-brand font-semibold tracking-wider"
                    data-oid="6-k5bin"
                  >
                    {username}
                  </span>
                </div>
              )}

              {showSettings && (
                <button
                  type="button"
                  onClick={() => navigate("/settings")}
                  aria-label="Settings"
                  className={cn(
                    "nav-btn inline-flex items-center justify-center whitespace-nowrap",
                    "nav-ui-fluid font-semibold tracking-wide text-[var(--nav-btn-text)]",
                    "bg-[var(--nav-btn-bg)] border border-[var(--nav-btn-border)]",
                    "hover:bg-[var(--nav-btn-bg-hover)] hover:border-[var(--nav-btn-border-hover)]",
                    "active:translate-y-px transition-all duration-200",
                  )}
                  data-oid="p:eh6bw"
                >
                  <Settings
                    className="nav-icon shrink-0 text-[var(--nav-btn-icon)]"
                    aria-hidden="true"
                    data-oid="s8j_75m"
                  />

                  <span
                    className="hidden sm:inline whitespace-nowrap"
                    data-oid="gf_rv:m"
                  >
                    Settings
                  </span>
                </button>
              )}

              <ThemeToggle showLabel data-oid="zsuxs6n" />

              <button
                type="button"
                onClick={onLogout}
                aria-label="Logout"
                className={cn(
                  "nav-btn inline-flex items-center justify-center whitespace-nowrap",
                  "nav-ui-fluid font-brand font-semibold tracking-wide text-[var(--nav-btn-text)]",
                  "bg-[var(--nav-btn-bg)] border border-[var(--nav-btn-border)]",
                  "hover:bg-[var(--nav-btn-bg-hover)] hover:border-[var(--nav-btn-border-danger-hover)]",
                  "active:translate-y-px transition-all duration-200",
                )}
                data-oid="0-jtd0c"
              >
                <LogOut
                  className="nav-icon shrink-0 text-[var(--nav-btn-icon-danger)]"
                  aria-hidden="true"
                  data-oid="jlxbevw"
                />

                <span
                  className="hidden sm:inline whitespace-nowrap"
                  data-oid="bhas7u6"
                >
                  Logout
                </span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </nav>
  );
}

function CyberPrismLogo({ className }: { className?: string }) {
  return (
    <svg
      className={cn("drop-shadow-lg", className)}
      viewBox="0 0 96 96"
      role="img"
      aria-label="Logo"
      data-oid="4nnbgu6"
    >
      <defs data-oid="8nhf6p2">
        <linearGradient
          id="cp_a"
          x1="0"
          y1="0"
          x2="1"
          y2="1"
          data-oid="-_xpsym"
        >
          <stop offset="0" stopColor="#32d6c6" data-oid="7qeipmf" />
          <stop offset="1" stopColor="#46b7ff" data-oid="16dww:g" />
        </linearGradient>
        <linearGradient
          id="cp_b"
          x1="1"
          y1="0"
          x2="0"
          y2="1"
          data-oid="aqaw5qj"
        >
          <stop offset="0" stopColor="#a855f7" data-oid="i39.l-y" />
          <stop offset="1" stopColor="#ff4fd8" data-oid="4xeiw9." />
        </linearGradient>
        <linearGradient
          id="cp_c"
          x1="0"
          y1="0"
          x2="0"
          y2="1"
          data-oid="_.h6e8."
        >
          <stop offset="0" stopColor="#8b5cf6" data-oid="an8o8-y" />
          <stop offset="1" stopColor="#22c55e" data-oid=".-mofzf" />
        </linearGradient>
        <linearGradient
          id="cp_d"
          x1="1"
          y1="0"
          x2="0"
          y2="1"
          data-oid="6q3hqng"
        >
          <stop offset="0" stopColor="#22c55e" data-oid="m0dqanz" />
          <stop offset="1" stopColor="#46b7ff" data-oid="7tvw.75" />
        </linearGradient>
        <filter
          id="cp_glow"
          x="-50%"
          y="-50%"
          width="200%"
          height="200%"
          data-oid="q9ub-1q"
        >
          <feGaussianBlur stdDeviation="3" result="blur" data-oid="ui:pwdo" />
          <feColorMatrix
            in="blur"
            type="matrix"
            values="
              1 0 0 0 0
              0 1 0 0 0
              0 0 1 0 0
              0 0 0 0.6 0"
            result="glow"
            data-oid="mt3skb4"
          />

          <feMerge data-oid="lx6zqh6">
            <feMergeNode in="glow" data-oid="x6uk50x" />
            <feMergeNode in="SourceGraphic" data-oid="pbvn:x7" />
          </feMerge>
        </filter>
      </defs>

      {/* 外层霓虹光晕 */}
      <g filter="url(#cp_glow)" opacity="0.95" data-oid="_vsfh.g">
        {/* 钻石外轮廓（对齐参考图的“钻石”形状） */}
        <polygon
          points="48,10 82,40 48,86 14,40"
          fill="rgba(178,139,255,0.10)"
          data-oid="gfua909"
        />

        {/* 4 个切面 */}
        <polygon
          points="48,10 14,40 48,48"
          fill="url(#cp_a)"
          data-oid="ifyhtkf"
        />

        <polygon
          points="48,10 82,40 48,48"
          fill="url(#cp_b)"
          data-oid="jh4q-:c"
        />

        <polygon
          points="14,40 48,86 48,48"
          fill="url(#cp_c)"
          data-oid="fuhycej"
        />

        <polygon
          points="82,40 48,86 48,48"
          fill="url(#cp_d)"
          data-oid="8gpvr18"
        />

        {/* 高光（上半部分轻微发亮） */}
        <polygon
          points="48,14 72,39 48,32 24,39"
          fill="rgba(255,255,255,0.22)"
          data-oid="5lp9r7s"
        />

        {/* 内部切线（更像切割钻石） */}
        <path
          d="M48 10 L48 86 M14 40 L82 40 M24 39 L48 48 L72 39"
          fill="none"
          stroke="rgba(255,255,255,0.20)"
          strokeWidth="1"
          data-oid="ls3a7qg"
        />

        {/* 外描边 */}
        <polygon
          points="48,10 82,40 48,86 14,40"
          fill="none"
          stroke="rgba(255,255,255,0.28)"
          strokeWidth="1"
          data-oid="po92g4t"
        />
      </g>
    </svg>
  );
}
