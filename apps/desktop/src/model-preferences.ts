import { useCallback, useEffect, useState } from "react";

export const MODEL_PREFERENCES_STORAGE_KEY = "aip.settings.models";
export const MODEL_PREFERENCES_EVENT = "aip-model-preferences-changed";
const MAX_MODEL_REFS = 64;
const MAX_MODEL_REF_LENGTH = 208;

export const MODEL_POLICY_MODES = [
  "auto",
  "quality",
  "speed",
  "manual",
] as const;
export type ModelPolicyMode = (typeof MODEL_POLICY_MODES)[number];

export type ModelPreferences = {
  hiddenModelRefs: string[];
  excludedModelRefs: string[];
  fallbackOnlyModelRefs: string[];
  preferredModelRef: string | null;
  policyMode: ModelPolicyMode;
};

export const DEFAULT_MODEL_PREFERENCES: ModelPreferences = {
  hiddenModelRefs: [],
  excludedModelRefs: [],
  fallbackOnlyModelRefs: [],
  preferredModelRef: null,
  policyMode: "auto",
};

function isBoundedModelRef(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_MODEL_REF_LENGTH &&
    !Array.from(value).some((character) => character.charCodeAt(0) < 32)
  );
}

function normalizeRefs(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return Array.from(
    new Set(value.filter(isBoundedModelRef).slice(0, MAX_MODEL_REFS)),
  );
}

export function normalizeModelPreferences(value: unknown): ModelPreferences {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return DEFAULT_MODEL_PREFERENCES;
  }
  const candidate = value as Partial<ModelPreferences>;
  return {
    hiddenModelRefs: normalizeRefs(candidate.hiddenModelRefs),
    excludedModelRefs: normalizeRefs(candidate.excludedModelRefs),
    fallbackOnlyModelRefs: normalizeRefs(candidate.fallbackOnlyModelRefs),
    preferredModelRef: isBoundedModelRef(candidate.preferredModelRef)
      ? candidate.preferredModelRef
      : null,
    policyMode: MODEL_POLICY_MODES.includes(
      candidate.policyMode as ModelPolicyMode,
    )
      ? (candidate.policyMode as ModelPolicyMode)
      : DEFAULT_MODEL_PREFERENCES.policyMode,
  };
}

export function readModelPreferences(): ModelPreferences {
  try {
    const stored = window.localStorage.getItem(MODEL_PREFERENCES_STORAGE_KEY);
    return stored === null
      ? DEFAULT_MODEL_PREFERENCES
      : normalizeModelPreferences(JSON.parse(stored));
  } catch {
    return DEFAULT_MODEL_PREFERENCES;
  }
}

export function writeModelPreferences(
  value: ModelPreferences,
): ModelPreferences {
  const normalized = normalizeModelPreferences(value);
  try {
    window.localStorage.setItem(
      MODEL_PREFERENCES_STORAGE_KEY,
      JSON.stringify(normalized),
    );
    window.dispatchEvent(new Event(MODEL_PREFERENCES_EVENT));
  } catch {
    // Preferences remain active for this render when storage is unavailable.
  }
  return normalized;
}

export function useModelPreferences(): readonly [
  ModelPreferences,
  (update: (current: ModelPreferences) => ModelPreferences) => void,
] {
  const [preferences, setPreferences] = useState(readModelPreferences);
  useEffect(() => {
    const sync = () => setPreferences(readModelPreferences());
    window.addEventListener(MODEL_PREFERENCES_EVENT, sync);
    window.addEventListener("storage", sync);
    return () => {
      window.removeEventListener(MODEL_PREFERENCES_EVENT, sync);
      window.removeEventListener("storage", sync);
    };
  }, []);
  const update = useCallback(
    (updater: (current: ModelPreferences) => ModelPreferences) => {
      setPreferences((current) => writeModelPreferences(updater(current)));
    },
    [],
  );
  return [preferences, update];
}
