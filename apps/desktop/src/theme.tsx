import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { AipSelect } from "./shared-controls";

export const THEME_STORAGE_KEY = "aip.ui.theme";

export const THEME_MODES = ["dark", "light", "system"] as const;
export type ThemeMode = (typeof THEME_MODES)[number];

export const RADIUS_PRESETS = {
  compact: {
    label: "Compacto",
    values: { xs: "4px", sm: "4px", md: "4px", lg: "8px", xl: "12px" },
  },
  standard: {
    label: "Padrão",
    values: { xs: "4px", sm: "8px", md: "8px", lg: "12px", xl: "16px" },
  },
  soft: {
    label: "Suave",
    values: { xs: "4px", sm: "8px", md: "12px", lg: "16px", xl: "24px" },
  },
} as const;
export type RadiusPreset = keyof typeof RADIUS_PRESETS;

export const UI_FONTS = {
  times: {
    label: "Times New Roman",
    stack: '"Times New Roman", Times, serif',
  },
  system: {
    label: "Sistema",
    stack:
      'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
  inter: {
    label: "Inter",
    stack:
      'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
  atkinson: {
    label: "Atkinson Hyperlegible",
    stack:
      '"Atkinson Hyperlegible", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
} as const;
export type UiFont = keyof typeof UI_FONTS;

export type ThemePreferences = {
  mode: ThemeMode;
  primaryColor: string;
  secondaryColor: string;
  radius: RadiusPreset;
  font: UiFont;
};

export const DEFAULT_THEME_PREFERENCES: ThemePreferences = {
  mode: "dark",
  primaryColor: "#d0aa72",
  secondaryColor: "#efd09b",
  radius: "standard",
  font: "times",
};

const LIGHT_TEXT = "#fffaf1";
const DARK_TEXT = "#241d14";

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isHexColor(value: unknown): value is string {
  return typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
}

export function normalizeHexColor(value: unknown, fallback: string): string {
  return isHexColor(value) ? value.toLowerCase() : fallback;
}

function channel(value: string): number {
  return Number.parseInt(value, 16) / 255;
}

function luminance(color: string): number {
  const red = channel(color.slice(1, 3));
  const green = channel(color.slice(3, 5));
  const blue = channel(color.slice(5, 7));
  return [red, green, blue]
    .map((component) =>
      component <= 0.03928
        ? component / 12.92
        : ((component + 0.055) / 1.055) ** 2.4,
    )
    .reduce((total, component, index) => {
      const weight = [0.2126, 0.7152, 0.0722][index] ?? 0;
      return total + component * weight;
    }, 0);
}

export function contrastRatio(first: string, second: string): number {
  if (!isHexColor(first) || !isHexColor(second)) return 1;
  const firstLuminance = luminance(first);
  const secondLuminance = luminance(second);
  const light = Math.max(firstLuminance, secondLuminance);
  const dark = Math.min(firstLuminance, secondLuminance);
  return (light + 0.05) / (dark + 0.05);
}

export function readableForeground(background: string): string {
  const color = normalizeHexColor(
    background,
    DEFAULT_THEME_PREFERENCES.primaryColor,
  );
  return contrastRatio(color, DARK_TEXT) >= contrastRatio(color, LIGHT_TEXT)
    ? DARK_TEXT
    : LIGHT_TEXT;
}

export function normalizeThemePreferences(value: unknown): ThemePreferences {
  if (!isRecord(value)) return DEFAULT_THEME_PREFERENCES;
  const mode = THEME_MODES.includes(value.mode as ThemeMode)
    ? (value.mode as ThemeMode)
    : DEFAULT_THEME_PREFERENCES.mode;
  const radius =
    typeof value.radius === "string" &&
    Object.hasOwn(RADIUS_PRESETS, value.radius)
      ? (value.radius as RadiusPreset)
      : DEFAULT_THEME_PREFERENCES.radius;
  const font =
    typeof value.font === "string" && Object.hasOwn(UI_FONTS, value.font)
      ? (value.font as UiFont)
      : DEFAULT_THEME_PREFERENCES.font;
  return {
    mode,
    primaryColor: normalizeHexColor(
      value.primaryColor,
      DEFAULT_THEME_PREFERENCES.primaryColor,
    ),
    secondaryColor: normalizeHexColor(
      value.secondaryColor,
      DEFAULT_THEME_PREFERENCES.secondaryColor,
    ),
    radius,
    font,
  };
}

function readStoredPreferences(): ThemePreferences {
  try {
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    return stored === null
      ? DEFAULT_THEME_PREFERENCES
      : normalizeThemePreferences(JSON.parse(stored));
  } catch {
    return DEFAULT_THEME_PREFERENCES;
  }
}

function systemTheme(): Exclude<ThemeMode, "system"> {
  return window.matchMedia?.("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
}

export function themeCssVariables(
  preferences: ThemePreferences,
  resolvedMode: Exclude<ThemeMode, "system">,
  reducedMotion: boolean,
): Record<string, string> {
  const radius = RADIUS_PRESETS[preferences.radius].values;
  const primary = normalizeHexColor(
    preferences.primaryColor,
    DEFAULT_THEME_PREFERENCES.primaryColor,
  );
  const secondary = normalizeHexColor(
    preferences.secondaryColor,
    DEFAULT_THEME_PREFERENCES.secondaryColor,
  );
  const palette =
    resolvedMode === "light"
      ? {
          canvas: "#f3f0ea",
          surface: "#fffdf8",
          raised: "#f8f4ed",
          soft: "#eee9df",
          mutedSurface: "#f5f0e7",
          border: "#d0c8bd",
          strongBorder: "#aa9f92",
          text: "#2c2925",
          mutedText: "#5f5951",
          subtleText: "#766f67",
          success: "#2f7650",
          warning: "#8a5a00",
          danger: "#a44f48",
        }
      : {
          canvas: "#121314",
          surface: "#1a1b1d",
          raised: "#222427",
          soft: "#292b2f",
          mutedSurface: "#202226",
          border: "#383b40",
          strongBorder: "#555a61",
          text: "#f3f1ec",
          mutedText: "#c1bdb5",
          subtleText: "#918d86",
          success: "#9ac6a4",
          warning: "#ddbd76",
          danger: "#e0a09a",
        };
  return {
    "--font-ui": UI_FONTS[preferences.font].stack,
    "--font-mono": 'Consolas, "Cascadia Code", monospace',
    "--font-size-body": "16px",
    "--line-height-body": "1.5",
    "--readability-measure": "68ch",
    "--type-meta": "12px",
    "--type-label": "14px",
    "--type-body": "16px",
    "--type-section": "18px",
    "--type-page": "22px",
    "--type-display": "28px",
    "--space-1": "4px",
    "--space-2": "8px",
    "--space-3": "12px",
    "--space-4": "16px",
    "--space-5": "20px",
    "--space-6": "24px",
    "--space-7": "32px",
    "--space-8": "40px",
    "--space-9": "48px",
    "--radius-xs": radius.xs,
    "--radius-sm": radius.sm,
    "--radius-md": radius.md,
    "--radius-lg": radius.lg,
    "--radius-xl": radius.xl,
    "--motion-fast": reducedMotion ? "0ms" : "120ms",
    "--motion-standard": reducedMotion ? "0ms" : "180ms",
    "--motion-idle": reducedMotion ? "0ms" : "2.4s",
    "--motion-in": reducedMotion ? "0ms" : "180ms",
    "--motion-out": reducedMotion ? "0ms" : "120ms",
    "--motion-state": reducedMotion ? "0ms" : "160ms",
    "--control-height": "40px",
    "--control-height-comfortable": "48px",
    "--focus-ring": `0 0 0 3px ${resolvedMode === "light" ? "#fffdf8" : "#121314"}, 0 0 0 5px ${primary}`,
    "--color-primary": primary,
    "--color-on-primary": readableForeground(primary),
    "--color-secondary": secondary,
    "--color-on-secondary": readableForeground(secondary),
    "--color-accent": primary,
    "--color-accent-strong": secondary,
    "--color-accent-ink": readableForeground(primary),
    "--color-background": palette.canvas,
    "--color-surface": palette.surface,
    "--color-surface-raised": palette.raised,
    "--color-surface-soft": palette.soft,
    "--color-surface-muted": palette.mutedSurface,
    "--color-surface-deep": palette.canvas,
    "--color-text": palette.text,
    "--color-text-strong": palette.text,
    "--color-text-muted": palette.mutedText,
    "--color-text-subtle": palette.subtleText,
    "--color-border": palette.border,
    "--color-border-strong": palette.strongBorder,
    "--color-success": palette.success,
    "--color-warning": palette.warning,
    "--color-danger": palette.danger,
    "--color-focus": primary,
    "--shadow-menu":
      resolvedMode === "light"
        ? "0 14px 32px rgba(44, 41, 37, 0.18)"
        : "0 14px 32px rgba(0, 0, 0, 0.42)",
    "--shadow-dialog":
      resolvedMode === "light"
        ? "0 24px 64px rgba(44, 41, 37, 0.24)"
        : "0 24px 64px rgba(0, 0, 0, 0.58)",
  };
}

type ThemeContextValue = {
  preferences: ThemePreferences;
  resolvedMode: Exclude<ThemeMode, "system">;
  reducedMotion: boolean;
  updatePreferences: (patch: Partial<ThemePreferences>) => void;
};

const ThemeContext = createContext<ThemeContextValue | null>(null);
const useIsomorphicLayoutEffect =
  typeof window === "undefined" ? useEffect : useLayoutEffect;

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState(readStoredPreferences);
  const [systemMode, setSystemMode] = useState(systemTheme);
  const [reducedMotion, setReducedMotion] = useState(
    () =>
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  );
  const resolvedMode =
    preferences.mode === "system" ? systemMode : preferences.mode;

  useEffect(() => {
    const query = window.matchMedia?.("(prefers-color-scheme: light)");
    if (query === undefined) return;
    const update = () => setSystemMode(query.matches ? "light" : "dark");
    update();
    query.addEventListener?.("change", update);
    return () => query.removeEventListener?.("change", update);
  }, []);

  useEffect(() => {
    const query = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    if (query === undefined) return;
    const update = () => setReducedMotion(query.matches);
    update();
    query.addEventListener?.("change", update);
    return () => query.removeEventListener?.("change", update);
  }, []);

  useIsomorphicLayoutEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolvedMode;
    root.dataset.themeMode = preferences.mode;
    root.dataset.motion = reducedMotion ? "reduced" : "full";
    root.style.colorScheme = resolvedMode;
    for (const [name, value] of Object.entries(
      themeCssVariables(preferences, resolvedMode, reducedMotion),
    )) {
      root.style.setProperty(name, value);
    }
    try {
      window.localStorage.setItem(
        THEME_STORAGE_KEY,
        JSON.stringify(preferences),
      );
    } catch {
      // UI preferences remain usable when storage is unavailable.
    }
  }, [preferences, reducedMotion, resolvedMode]);

  const updatePreferences = useCallback((patch: Partial<ThemePreferences>) => {
    setPreferences((current) =>
      normalizeThemePreferences({ ...current, ...patch }),
    );
  }, []);

  const value = useMemo(
    () => ({ preferences, resolvedMode, reducedMotion, updatePreferences }),
    [preferences, resolvedMode, reducedMotion, updatePreferences],
  );
  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (value === null)
    throw new Error("useTheme must be used within ThemeProvider");
  return value;
}

export function ThemeControls() {
  const { preferences, updatePreferences, resolvedMode, reducedMotion } =
    useTheme();
  return (
    <fieldset className="theme-controls">
      <legend>Aparência da interface</legend>
      <AipSelect
        id="theme-mode"
        label="Modo de aparência"
        value={preferences.mode}
        options={[
          { value: "dark", label: "Escuro" },
          { value: "light", label: "Claro" },
          { value: "system", label: "Sistema" },
        ]}
        onChange={(mode) => {
          if (THEME_MODES.includes(mode as ThemeMode))
            updatePreferences({ mode: mode as ThemeMode });
        }}
      />
      <div className="theme-color-grid">
        <label>
          Cor primária
          <input
            type="color"
            value={preferences.primaryColor}
            onChange={(event) =>
              updatePreferences({ primaryColor: event.target.value })
            }
          />
        </label>
        <label>
          Cor secundária
          <input
            type="color"
            value={preferences.secondaryColor}
            onChange={(event) =>
              updatePreferences({ secondaryColor: event.target.value })
            }
          />
        </label>
      </div>
      <AipSelect
        id="theme-radius"
        label="Raio global"
        value={preferences.radius}
        options={Object.entries(RADIUS_PRESETS).map(([value, preset]) => ({
          value,
          label: preset.label,
        }))}
        onChange={(radius) => {
          if (Object.hasOwn(RADIUS_PRESETS, radius))
            updatePreferences({ radius: radius as RadiusPreset });
        }}
      />
      <AipSelect
        id="theme-font"
        label="Fonte da interface"
        value={preferences.font}
        options={Object.entries(UI_FONTS).map(([value, font]) => ({
          value,
          label: font.label,
        }))}
        onChange={(font) => {
          if (Object.hasOwn(UI_FONTS, font))
            updatePreferences({ font: font as UiFont });
        }}
      />
      <small className="readable-helper" role="status">
        Tema ativo: {resolvedMode === "light" ? "claro" : "escuro"}. Animações:{" "}
        {reducedMotion ? "reduzidas" : "normais"}.
      </small>
    </fieldset>
  );
}
