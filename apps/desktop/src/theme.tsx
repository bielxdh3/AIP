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

export const THEME_STORAGE_KEY = "aip.ui.theme";

export const THEME_MODES = ["dark", "light", "system"] as const;
export type ThemeMode = (typeof THEME_MODES)[number];

export const RADIUS_PRESETS = {
  compact: {
    label: "Compacto",
    values: { xs: "2px", sm: "4px", md: "6px", lg: "8px", xl: "10px" },
  },
  standard: {
    label: "Padrão",
    values: { xs: "3px", sm: "5px", md: "7px", lg: "9px", xl: "14px" },
  },
  soft: {
    label: "Suave",
    values: { xs: "5px", sm: "8px", md: "12px", lg: "16px", xl: "20px" },
  },
} as const;
export type RadiusPreset = keyof typeof RADIUS_PRESETS;

export const UI_FONTS = {
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
  primaryColor: "#57d8bd",
  secondaryColor: "#74c7b4",
  radius: "standard",
  font: "inter",
};

const LIGHT_TEXT = "#ffffff";
const DARK_TEXT = "#07120f";

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
  return {
    "--font-ui": UI_FONTS[preferences.font].stack,
    "--font-mono": 'Consolas, "Cascadia Code", monospace',
    "--font-size-body": "16px",
    "--line-height-body": "1.5",
    "--readability-measure": "70ch",
    "--radius-xs": radius.xs,
    "--radius-sm": radius.sm,
    "--radius-md": radius.md,
    "--radius-lg": radius.lg,
    "--radius-xl": radius.xl,
    "--motion-fast": reducedMotion ? "0ms" : "120ms",
    "--motion-standard": reducedMotion ? "0ms" : "180ms",
    "--motion-idle": reducedMotion ? "0ms" : "2.4s",
    "--color-primary": normalizeHexColor(
      preferences.primaryColor,
      DEFAULT_THEME_PREFERENCES.primaryColor,
    ),
    "--color-on-primary": readableForeground(preferences.primaryColor),
    "--color-secondary": normalizeHexColor(
      preferences.secondaryColor,
      DEFAULT_THEME_PREFERENCES.secondaryColor,
    ),
    "--color-on-secondary": readableForeground(preferences.secondaryColor),
    "--color-background": resolvedMode === "light" ? "#f4f7f8" : "#0b0d11",
    "--color-surface": resolvedMode === "light" ? "#ffffff" : "#11161c",
    "--color-surface-raised": resolvedMode === "light" ? "#eef3f5" : "#171d24",
    "--color-surface-deep": resolvedMode === "light" ? "#e7edef" : "#0b0f14",
    "--color-text": resolvedMode === "light" ? "#16232b" : "#e7eaf0",
    "--color-text-strong": resolvedMode === "light" ? "#0c171d" : "#edf0f5",
    "--color-text-muted": resolvedMode === "light" ? "#42525a" : "#aeb6c2",
    "--color-text-subtle": resolvedMode === "light" ? "#5a6a72" : "#68717e",
    "--color-border": resolvedMode === "light" ? "#c8d5d9" : "#2c3442",
    "--color-border-strong": resolvedMode === "light" ? "#9fb1b8" : "#46515f",
    "--color-success": resolvedMode === "light" ? "#087f68" : "#74c7b4",
    "--color-warning": resolvedMode === "light" ? "#8a5a00" : "#d7c18f",
    "--color-danger": resolvedMode === "light" ? "#b42318" : "#ef8e7e",
    "--color-focus": resolvedMode === "light" ? "#075e54" : "#57d8bd",
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
      <label>
        Modo de aparência
        <select
          value={preferences.mode}
          onChange={(event) =>
            updatePreferences({ mode: event.target.value as ThemeMode })
          }
        >
          <option value="dark">Escuro</option>
          <option value="light">Claro</option>
          <option value="system">Sistema</option>
        </select>
      </label>
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
      <label>
        Raio global
        <select
          value={preferences.radius}
          onChange={(event) =>
            updatePreferences({ radius: event.target.value as RadiusPreset })
          }
        >
          {Object.entries(RADIUS_PRESETS).map(([value, preset]) => (
            <option key={value} value={value}>
              {preset.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        Fonte da interface
        <select
          value={preferences.font}
          onChange={(event) =>
            updatePreferences({ font: event.target.value as UiFont })
          }
        >
          {Object.entries(UI_FONTS).map(([value, font]) => (
            <option key={value} value={value}>
              {font.label}
            </option>
          ))}
        </select>
      </label>
      <small role="status">
        Tema ativo: {resolvedMode === "light" ? "claro" : "escuro"}. Animações:{" "}
        {reducedMotion ? "reduzidas" : "normais"}.
      </small>
    </fieldset>
  );
}
