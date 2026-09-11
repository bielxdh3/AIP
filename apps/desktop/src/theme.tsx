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
export const THEME_MODES = [
  "dark",
  "paper",
  "graphite",
  "night",
  "sepia",
  "custom",
] as const;
export type ThemeMode = (typeof THEME_MODES)[number];
export type BuiltInThemeMode = Exclude<ThemeMode, "custom">;
export const THEME_LABELS: Record<ThemeMode, string> = {
  dark: "Escuro",
  paper: "Papel quente",
  graphite: "Grafite",
  night: "Azul noite",
  sepia: "Sépia",
  custom: "Personalizado",
};

export const RADIUS_PRESETS = {
  compact: {
    label: "Compacto",
    values: { xs: "4px", sm: "4px", md: "4px", lg: "8px", xl: "12px" },
  },
  soft: {
    label: "Suave",
    values: { xs: "4px", sm: "8px", md: "12px", lg: "16px", xl: "24px" },
  },
} as const;
export type RadiusPreset = keyof typeof RADIUS_PRESETS;

export const UI_FONTS = {
  inter: {
    label: "Inter",
    group: "Sans",
    stack:
      'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
  segoe: {
    label: "Segoe UI",
    group: "Sans",
    stack: '"Segoe UI", ui-sans-serif, system-ui, sans-serif',
  },
  arial: { label: "Arial", group: "Sans", stack: "Arial, sans-serif" },
  verdana: { label: "Verdana", group: "Sans", stack: "Verdana, sans-serif" },
  tahoma: { label: "Tahoma", group: "Sans", stack: "Tahoma, sans-serif" },
  trebuchet: {
    label: "Trebuchet MS",
    group: "Sans",
    stack: '"Trebuchet MS", sans-serif',
  },
  calibri: { label: "Calibri", group: "Sans", stack: "Calibri, sans-serif" },
  times: {
    label: "Times New Roman",
    group: "Serif",
    stack: '"Times New Roman", Times, serif',
  },
  georgia: { label: "Georgia", group: "Serif", stack: "Georgia, serif" },
  cambria: { label: "Cambria", group: "Serif", stack: "Cambria, serif" },
  palatino: {
    label: "Palatino Linotype",
    group: "Serif",
    stack: '"Palatino Linotype", Palatino, serif',
  },
  garamond: { label: "Garamond", group: "Serif", stack: "Garamond, serif" },
  consolas: {
    label: "Consolas",
    group: "Mono",
    stack: 'Consolas, "Cascadia Code", monospace',
  },
  courier: {
    label: "Courier New",
    group: "Mono",
    stack: '"Courier New", monospace',
  },
  atkinson: {
    label: "Atkinson Hyperlegible",
    group: "Acessibilidade",
    stack:
      '"Atkinson Hyperlegible", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
} as const;
export type UiFont = keyof typeof UI_FONTS;

export type ThemePalette = {
  canvas: string;
  surface: string;
  raised: string;
  soft: string;
  mutedSurface: string;
  border: string;
  strongBorder: string;
  text: string;
  mutedText: string;
  subtleText: string;
  success: string;
  warning: string;
  danger: string;
};
type PaletteOverrides = Partial<ThemePalette>;

const PRESETS: Record<BuiltInThemeMode, ThemePalette> = {
  dark: {
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
  },
  paper: {
    canvas: "#c9b9a4",
    surface: "#d7c7b3",
    raised: "#e1d1bc",
    soft: "#bda990",
    mutedSurface: "#cfbea8",
    border: "#927d66",
    strongBorder: "#705b47",
    text: "#2d241b",
    mutedText: "#4c3d2e",
    subtleText: "#564333",
    success: "#23633e",
    warning: "#704500",
    danger: "#8f3f3a",
  },
  graphite: {
    canvas: "#17191d",
    surface: "#202329",
    raised: "#2a2e35",
    soft: "#343a43",
    mutedSurface: "#252a31",
    border: "#454c57",
    strongBorder: "#687381",
    text: "#f0f2f4",
    mutedText: "#c1c7cf",
    subtleText: "#929aa5",
    success: "#94d1ad",
    warning: "#e4c277",
    danger: "#edaaa5",
  },
  night: {
    canvas: "#0d1726",
    surface: "#122238",
    raised: "#1a2d46",
    soft: "#233c5b",
    mutedSurface: "#172a42",
    border: "#35516f",
    strongBorder: "#55789b",
    text: "#eff6ff",
    mutedText: "#bdd0e4",
    subtleText: "#8fa9c4",
    success: "#8fd4b0",
    warning: "#e5c274",
    danger: "#efaaa6",
  },
  sepia: {
    canvas: "#2a211b",
    surface: "#35291f",
    raised: "#423326",
    soft: "#55432f",
    mutedSurface: "#3c2f24",
    border: "#6e5940",
    strongBorder: "#937655",
    text: "#f9ecd5",
    mutedText: "#dbc6a7",
    subtleText: "#b79d7b",
    success: "#a5c995",
    warning: "#e1c076",
    danger: "#e2a29a",
  },
};
export const THEME_PRESETS = PRESETS;
const PALETTE_KEYS = Object.keys(PRESETS.dark) as Array<keyof ThemePalette>;

export type ThemePreferences = {
  mode: ThemeMode;
  primaryColor: string;
  secondaryColor: string;
  radius: RadiusPreset;
  font: UiFont;
  customColors: ThemePalette;
  overrides: Partial<Record<BuiltInThemeMode, PaletteOverrides>>;
};
export const DEFAULT_THEME_PREFERENCES: ThemePreferences = {
  mode: "dark",
  primaryColor: "#d0aa72",
  secondaryColor: "#efd09b",
  radius: "compact",
  font: "times",
  customColors: { ...PRESETS.dark },
  overrides: {},
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
function normalizePalette(
  value: unknown,
  fallback: ThemePalette,
): ThemePalette {
  if (!isRecord(value)) return { ...fallback };
  return Object.fromEntries(
    PALETTE_KEYS.map((key) => [
      key,
      normalizeHexColor(value[key], fallback[key]),
    ]),
  ) as ThemePalette;
}
function normalizeOverrides(value: unknown): ThemePreferences["overrides"] {
  if (!isRecord(value)) return {};
  const overrides: ThemePreferences["overrides"] = {};
  for (const mode of Object.keys(PRESETS) as BuiltInThemeMode[]) {
    const source = value[mode];
    if (!isRecord(source)) continue;
    const normalized: PaletteOverrides = {};
    for (const key of PALETTE_KEYS)
      if (isHexColor(source[key])) normalized[key] = source[key].toLowerCase();
    if (Object.keys(normalized).length > 0) overrides[mode] = normalized;
  }
  return overrides;
}
export function contrastRatio(first: string, second: string): number {
  if (!isHexColor(first) || !isHexColor(second)) return 1;
  const channel = (value: string) => {
    const parsed = Number.parseInt(value, 16) / 255;
    return parsed <= 0.03928
      ? parsed / 12.92
      : ((parsed + 0.055) / 1.055) ** 2.4;
  };
  const luminance = (color: string) =>
    [
      channel(color.slice(1, 3)),
      channel(color.slice(3, 5)),
      channel(color.slice(5, 7)),
    ].reduce(
      (total, component, index) =>
        total + component * [0.2126, 0.7152, 0.0722][index]!,
      0,
    );
  const light = Math.max(luminance(first), luminance(second));
  const dark = Math.min(luminance(first), luminance(second));
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
  const legacyMode =
    value.mode === "system"
      ? "dark"
      : value.mode === "light"
        ? "paper"
        : value.mode;
  const mode = THEME_MODES.includes(legacyMode as ThemeMode)
    ? (legacyMode as ThemeMode)
    : DEFAULT_THEME_PREFERENCES.mode;
  const radius =
    value.radius === "standard"
      ? "compact"
      : typeof value.radius === "string" &&
          Object.hasOwn(RADIUS_PRESETS, value.radius)
        ? (value.radius as RadiusPreset)
        : DEFAULT_THEME_PREFERENCES.radius;
  const font =
    value.font === "system"
      ? "times"
      : typeof value.font === "string" && Object.hasOwn(UI_FONTS, value.font)
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
    customColors: normalizePalette(value.customColors, PRESETS.dark),
    overrides: normalizeOverrides(value.overrides),
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
function activePalette(preferences: ThemePreferences): ThemePalette {
  if (preferences.mode === "custom") return preferences.customColors;
  return {
    ...PRESETS[preferences.mode],
    ...preferences.overrides[preferences.mode],
  };
}

export function themeCssVariables(
  preferences: ThemePreferences,
  resolvedMode: ThemeMode,
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
  const palette = activePalette(preferences);
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
    "--focus-ring": `0 0 0 3px ${palette.canvas}, 0 0 0 5px ${primary}`,
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
      resolvedMode === "paper"
        ? "0 14px 32px rgba(44, 41, 37, 0.22)"
        : "0 14px 32px rgba(0, 0, 0, 0.42)",
    "--shadow-dialog":
      resolvedMode === "paper"
        ? "0 24px 64px rgba(44, 41, 37, 0.30)"
        : "0 24px 64px rgba(0, 0, 0, 0.58)",
  };
}

type ThemeContextValue = {
  preferences: ThemePreferences;
  resolvedMode: ThemeMode;
  reducedMotion: boolean;
  updatePreferences: (patch: Partial<ThemePreferences>) => void;
};
const ThemeContext = createContext<ThemeContextValue | null>(null);
const useIsomorphicLayoutEffect =
  typeof window === "undefined" ? useEffect : useLayoutEffect;
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState(readStoredPreferences);
  const [reducedMotion, setReducedMotion] = useState(
    () =>
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  );
  const resolvedMode = preferences.mode;
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
    root.style.colorScheme = resolvedMode === "paper" ? "light" : "dark";
    for (const [name, value] of Object.entries(
      themeCssVariables(preferences, resolvedMode, reducedMotion),
    ))
      root.style.setProperty(name, value);
    try {
      window.localStorage.setItem(
        THEME_STORAGE_KEY,
        JSON.stringify(preferences),
      );
    } catch {
      /* storage is optional */
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
function isModified(
  preferences: ThemePreferences,
  mode: BuiltInThemeMode,
): boolean {
  return Object.keys(preferences.overrides[mode] ?? {}).length > 0;
}
function isCustomModified(preferences: ThemePreferences): boolean {
  return PALETTE_KEYS.some(
    (key) => preferences.customColors[key] !== PRESETS.dark[key],
  );
}

export function ThemeControls() {
  const { preferences, updatePreferences, resolvedMode, reducedMotion } =
    useTheme();
  const palette = activePalette(preferences);
  const setPaletteColor = (key: keyof ThemePalette, value: string) =>
    preferences.mode === "custom"
      ? updatePreferences({
          customColors: { ...preferences.customColors, [key]: value },
        })
      : updatePreferences({
          overrides: {
            ...preferences.overrides,
            [preferences.mode]: {
              ...preferences.overrides[preferences.mode],
              [key]: value,
            },
          },
        });
  const restore = () => {
    if (preferences.mode === "custom")
      updatePreferences({ customColors: { ...PRESETS.dark } });
    else {
      const overrides = { ...preferences.overrides };
      delete overrides[preferences.mode];
      updatePreferences({ overrides });
    }
  };
  const colorControl = (key: keyof ThemePalette, label: string) => (
    <label key={key}>
      {label}
      <input
        type="color"
        value={palette[key]}
        aria-label={label}
        onChange={(event) => setPaletteColor(key, event.target.value)}
      />
    </label>
  );
  const modeOptions = THEME_MODES.map((mode) => ({
    value: mode,
    label: `${THEME_LABELS[mode]}${
      mode === "custom"
        ? isCustomModified(preferences)
          ? " — modificado"
          : ""
        : isModified(preferences, mode)
          ? " — modificado"
          : ""
    }`,
  }));
  const fontOptions = Object.entries(UI_FONTS).map(([value, font]) => ({
    value,
    label: `${font.group}: ${font.label}`,
  }));
  return (
    <fieldset className="theme-controls">
      <legend>Aparência da interface</legend>
      <AipSelect
        id="theme-mode"
        label="Tema"
        value={preferences.mode}
        options={modeOptions}
        onChange={(mode) => {
          if (THEME_MODES.includes(mode as ThemeMode))
            updatePreferences({ mode: mode as ThemeMode });
        }}
      />
      <div className="theme-color-grid">
        <label>
          Cor de destaque
          <input
            type="color"
            value={preferences.primaryColor}
            aria-label="Cor de destaque"
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
            aria-label="Cor secundária"
            onChange={(event) =>
              updatePreferences({ secondaryColor: event.target.value })
            }
          />
        </label>
        {colorControl("canvas", "Fundo principal")}
        {colorControl("surface", "Superfície")}
        {colorControl("text", "Texto principal")}
        {colorControl("mutedText", "Texto secundário")}
        {colorControl("border", "Bordas")}
      </div>
      <div className="theme-controls-row">
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
          options={fontOptions}
          onChange={(font) => {
            if (Object.hasOwn(UI_FONTS, font))
              updatePreferences({ font: font as UiFont });
          }}
        />
      </div>
      {(
        preferences.mode === "custom"
          ? isCustomModified(preferences)
          : isModified(preferences, preferences.mode as BuiltInThemeMode)
      ) ? (
        <button
          type="button"
          className="secondary-action theme-reset"
          onClick={restore}
        >
          Restaurar padrão
        </button>
      ) : null}
      <small className="readable-helper" role="status">
        {THEME_LABELS[resolvedMode]}
        {(
          resolvedMode === "custom"
            ? isCustomModified(preferences)
            : isModified(preferences, resolvedMode as BuiltInThemeMode)
        )
          ? " — modificado"
          : ""}
        . Animações: {reducedMotion ? "reduzidas" : "normais"}.
      </small>
    </fieldset>
  );
}
