//! # Theme Store
//!
//! 管理应用主题（深色/浅色模式）的状态和持久化。

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Theme = 'light' | 'dark' | 'purple' | 'terminal' | 'portfolio';

interface ThemeState {
  theme: Theme;
  effectiveTheme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

function applyTheme(effectiveTheme: Theme) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.setAttribute('data-theme', effectiveTheme);
  root.classList.remove('light', 'purple', 'terminal', 'portfolio');
  if (effectiveTheme === 'light') {
    root.classList.remove('dark');
    root.classList.add('light');
    return;
  }

  root.classList.add('dark');
  if (effectiveTheme === 'purple') root.classList.add('purple');
  if (effectiveTheme === 'terminal') root.classList.add('terminal');
  if (effectiveTheme === 'portfolio') root.classList.add('portfolio');
}

function resolveMacOSTitlebarColor(): string | [number, number, number] | null {
  if (typeof document === 'undefined') return null;
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue('--macos-titlebar-bg')
    .trim();
  if (!value) return null;
  if (value.startsWith('#')) return value;
  const rgb = value.match(/^rgba?\((\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
  if (!rgb) return null;
  return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])];
}

async function applyMacOSWindowBackground() {
  if (typeof window === 'undefined') return;
  const isMacOS = navigator.userAgent.includes('Mac OS X');
  if (!isMacOS) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const color = resolveMacOSTitlebarColor();
    if (!color) return;
    await getCurrentWindow().setBackgroundColor(color);
  } catch {
    return;
  }
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => {
      // 初始化时应用主题
      const initialTheme: Theme = 'dark';
      const initialEffectiveTheme: Theme = initialTheme;
      applyTheme(initialEffectiveTheme);
      void applyMacOSWindowBackground();

      return {
        theme: initialTheme,
        effectiveTheme: initialEffectiveTheme,
        setTheme: (_theme: Theme) => {
          const theme = _theme;
          const effectiveTheme: Theme = theme;
          applyTheme(effectiveTheme);
          void applyMacOSWindowBackground();
          set({ theme, effectiveTheme });
        },
        toggleTheme: () => {
          const current = get().effectiveTheme;
          const nextTheme: Theme =
            current === 'dark'
              ? 'light'
              : current === 'light'
                ? 'purple'
                : current === 'purple'
                  ? 'terminal'
                  : current === 'terminal'
                    ? 'portfolio'
                    : 'dark';
          const effectiveTheme: Theme = nextTheme;
          applyTheme(effectiveTheme);
          void applyMacOSWindowBackground();
          set({ theme: nextTheme, effectiveTheme });
        },
      };
    },
    {
      name: 'theme-storage',
      onRehydrateStorage: () => (state) => {
        if (state) {
          const effectiveTheme: Theme = state.theme;
          applyTheme(effectiveTheme);
          void applyMacOSWindowBackground();
          state.effectiveTheme = effectiveTheme;
        }
      },
    }
  )
);
