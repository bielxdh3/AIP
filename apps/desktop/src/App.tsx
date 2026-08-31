import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createPortal } from "react-dom";
import {
  parseProviderSnapshot,
  parseLocalProviders,
  parseScreenVisionProviderStatus,
  parseVoiceDevice,
  parseVoiceProviderStatus,
} from "@aip/contracts";
import type {
  AppSnapshot,
  AgentMemory,
  AgentConversationInspection,
  AgentConversationSummary,
  AgentSimulatedState,
  CognitiveEventExplanation,
  CognitiveEventSummary,
  CognitiveGoalApprovalRequest,
  CognitiveGoal,
  FictionalActivity,
  FictionalActivityRequest,
  FictionalActivityStatusRequest,
  CognitiveGoalStatusRequest,
  CognitiveCandidate,
  CognitiveCandidateRequest,
  CognitiveCandidateRejectionRequest,
  CognitiveResourceJob,
  CognitiveOpinion,
  CognitiveOpinionRecalculationRequest,
  CognitiveOpinionStatusRequest,
  CognitiveTrait,
  ConversationMessage,
  ConversationInterruptRequest,
  ConversationPolicy,
  ConversationPolicyRequest,
  ConversationStartRequest,
  GoalRequest,
  HeavyGenerationRequest,
  OpinionCandidateRequest,
  OpinionEvidenceCorrectionRequest,
  PhaseOneConversation,
  PublicConversationTurnRequest,
  ProvisionalAgent,
  ResourceJobCompletionRequest,
  RelationshipCandidateRequest,
  RelationshipResetRequest,
  RelationshipRollbackRequest,
  RelationshipState,
  CustomVoiceConsentRequest,
  ExtensionAuditRecord,
  ExtensionCapability,
  ExtensionCatalogEntry,
  ExtensionImportRequest,
  ExtensionManifest,
  ExtensionInstruction,
  ExtensionExecutionResult,
  ExtensionProposal,
  ExtensionSourceKind,
  VoiceSettings,
  VoiceDevice,
  VoiceSettingsRequest,
  VoiceCaptureRuntimeRequest,
  VoiceOperationCancellationRequest,
  VoiceRuntimeSynthesisResult,
  VoiceRuntimeTranscriptionResult,
  VoiceRuntimeWakeWordResult,
  VoiceProviderStatus,
  LocalProvider,
  LocalProviderKind,
  LocalProviderRequest,
  LocalProviderIdRequest,
  VoiceSynthesisRequest,
  VoiceSynthesisResult,
  VoiceTranscriptionRequest,
  VoiceTranscriptionResult,
  ScreenVisionAuditRecord,
  ScreenVisionFixture,
  ScreenVisionHypothesis,
  ScreenVisionJob,
  ScreenVisionPermission,
  ScreenVisionPrivacyPolicy,
  ScreenVisionProviderStatus,
  ScreenVisionSession,
  CompanionAuditRecord,
  CompanionDevice,
  CompanionHistoryRecord,
  CompanionKeyRotation,
  CompanionQueueItem,
  CompanionRevocation,
  CompanionSession,
  CompanionSessionProof,
  GatewayAccount,
  GatewayAuditRecord,
  GatewayProtocolInfo,
  GatewayRecovery,
  GatewayRevocation,
  GatewaySession,
  GatewaySessionProof,
  GatewayTransfer,
  ToolAction,
  ToolActionInput,
  ToolManifest,
  ToolPermission,
  ToolSession,
  ProviderSnapshot,
  OllamaModel,
  PhaseOneState,
  WorkspaceRoot,
} from "@aip/contracts";
import {
  COMPANION_FIXTURE_APP_VERSION,
  COMPANION_FIXTURE_DEVICE_ID,
  COMPANION_FIXTURE_FINGERPRINT,
  COMPANION_FIXTURE_PAIRING_NONCE,
  COMPANION_PROTOCOL_VERSION,
  GATEWAY_FIXTURE_APP_VERSION,
  GATEWAY_FIXTURE_AUTH_PROOF_METADATA,
  GATEWAY_FIXTURE_CLIENT_ID,
  GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,
  GATEWAY_FIXTURE_RECOVERY_TARGET,
  GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,
  GATEWAY_PROTOCOL_VERSION,
  parseCognitiveError,
  parseCompanionAudit,
  parseCompanionDevice,
  parseCompanionDevices,
  parseCompanionHistory,
  parseCompanionKeyRotations,
  parseCompanionKeyRotation,
  parseCompanionQueue,
  parseCompanionQueueItem,
  parseCompanionRevocations,
  parseCompanionRevocation,
  parseCompanionSession,
  parseCompanionSessions,
  parseGatewayAccounts,
  parseGatewayAudit,
  parseGatewayProtocolInfo,
  parseGatewayRecovery,
  parseGatewayRecoveries,
  parseGatewayRevocation,
  parseGatewayRevocations,
  parseGatewaySession,
  parseGatewaySessions,
  parseGatewayTransfer,
  parseGatewayTransfers,
  parseExtensionAudit,
  parseExtensionCatalog,
  parseExtensionProposals,
  parseExtensionPackage,
  parseExtensionExecutionResult,
  parseScreenVisionAudit,
  parseScreenVisionAnalysisResult,
  parseScreenVisionFixtures,
  parseScreenVisionJobs,
  parseScreenVisionSessions,
  parseToolAction,
  parseToolAudit,
  parseToolCatalog,
  parseToolSession,
  parseToolSessions,
  parseWorkspaceRoots,
  parseWorkspaceRoot,
} from "@aip/contracts";
import AgentSprite from "./components/AgentSprite";
import {
  blockedSendCopy,
  canDraftConversationMessage,
  canSendConversationMessage,
  canRequestCancellation,
  conversationOverrideArguments,
  messageStatusCopy,
  providerRecoveryCopy,
  providerStatusCopy,
  requestForAgent,
} from "./conversation-state";
import {
  localizedCanonicalValue,
  profileCanonicalOptions,
  type ProfileCanonicalField,
} from "./profile-localization";
import {
  isNearConversationBottom,
  shouldScrollConversationToBottom,
} from "./conversation-scroll";
import {
  OPEN_AGENT_CONVERSATIONS_EVENT,
  type OpenAgentConversationsPayload,
} from "./agent-navigation";
import { usePhaseOne } from "./use-phase-one";
import { createListenerRegistration } from "./listener-lifecycle";
import {
  nextLayerId,
  floodFillLayer,
  paintPixelLayer,
  parsePixelDocument,
  rgbaToHex,
  selectionRectangle,
  updatePixelLayer,
  type PixelDocument,
  type PixelSelection,
} from "./pixel-document";
import "./App.css";

const runtimeLabels: Record<AppSnapshot["runtime"]["state"], string> = {
  stopped: "Runtime parado",
  starting: "Runtime iniciando",
  ready: "Runtime local pronto",
  unavailable: "Runtime de IA indisponível",
  crashed: "Runtime interrompido",
  safe_mode: "Runtime desativado pelo modo seguro",
};

const initialTraits = [
  ["curiosity", "Curiosidade"],
  ["sociability", "Sociabilidade"],
  ["criticality", "Criticidade"],
  ["spontaneity", "Espontaneidade"],
  ["affection", "Afetividade"],
  ["autonomy", "Autonomia"],
] as const;

function traitValues(source: string): Record<string, number> {
  try {
    const value: unknown = JSON.parse(source);
    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      return Object.fromEntries(
        Object.entries(value).filter(
          ([, trait]) => typeof trait === "number" && Number.isFinite(trait),
        ),
      );
    }
  } catch {
    /* Rust remains the authoritative validator. */
  }
  return {};
}

function updateInitialTrait(
  agent: ProvisionalAgent,
  key: string,
  value: number,
) {
  const traits = traitValues(agent.traitsJson);
  traits[key] = Math.max(0, Math.min(100, value));
  return { ...agent, traitsJson: JSON.stringify(traits) };
}

function withInitialTraitDefaults(agent: ProvisionalAgent): ProvisionalAgent {
  const traits = traitValues(agent.traitsJson);
  for (const [key] of initialTraits) traits[key] ??= 50;
  return { ...agent, traitsJson: JSON.stringify(traits) };
}

type ProfileDraftUpdater = (current: ProvisionalAgent) => ProvisionalAgent;

function profileDraftIsDirty(
  draft: ProvisionalAgent,
  persisted: ProvisionalAgent,
  fictiveAgeText: string,
): boolean {
  return (
    JSON.stringify(draft) !== JSON.stringify(persisted) ||
    fictiveAgeText !== String(persisted.fictiveAge)
  );
}

function validCalendarDate(value: string): boolean {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (match === null) return false;
  const [, yearText = "", monthText = "", dayText = ""] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}

function profileValidationError(agent: ProvisionalAgent): string | null {
  if (!agent.name.trim() || !agent.species.trim() || !agent.pronouns.trim())
    return "Preencha nome, espécie e pronomes.";
  if (!validCalendarDate(agent.birthday))
    return "Informe uma data de aniversário válida.";
  if (!agent.ageCategory.trim()) return "Informe a categoria de idade.";
  if (
    !Number.isFinite(agent.fictiveAge) ||
    !Number.isInteger(agent.fictiveAge) ||
    agent.fictiveAge < 0 ||
    agent.fictiveAge > 10000
  )
    return "Informe uma idade fictícia entre 0 e 10000.";
  if (
    initialTraits.some(([key]) => {
      const value = traitValues(agent.traitsJson)[key];
      return (
        value !== undefined &&
        (!Number.isFinite(value) || value < 0 || value > 100)
      );
    })
  )
    return "Cada traço deve ser um número entre 0 e 100.";
  return null;
}

function isHumanCompatibleSpecies(species: string): boolean {
  return ["human", "human-compatible"].includes(species.trim());
}

function isoDate(year: number, month: number, day: number): string {
  return `${year.toString().padStart(4, "0")}-${(month + 1)
    .toString()
    .padStart(2, "0")}-${day.toString().padStart(2, "0")}`;
}

function dateParts(value: string): [number, number, number] | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (match === null) return null;
  return [Number(match[1]), Number(match[2]) - 1, Number(match[3])];
}

function daysInMonth(year: number, month: number): number {
  return new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
}

function DatePicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const initial = dateParts(value) ?? [2000, 0, 1];
  const [open, setOpen] = useState(false);
  const [view, setView] = useState({ year: initial[0], month: initial[1] });
  const [yearText, setYearText] = useState(String(initial[0]));
  const pickerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const dayRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const popoverId = useId();
  const selected = dateParts(value);
  const selectedDate = selected === null ? null : isoDate(...selected);
  const firstWeekday = new Date(Date.UTC(view.year, view.month, 1)).getUTCDay();
  const monthLabel = new Intl.DateTimeFormat("pt-BR", {
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(Date.UTC(view.year, view.month, 1)));
  const days = Array.from(
    { length: daysInMonth(view.year, view.month) },
    (_, index) => index + 1,
  );

  useEffect(() => {
    if (!open) return;
    function closeOnOutside(event: PointerEvent) {
      if (!pickerRef.current?.contains(event.target as Node)) setOpen(false);
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
        triggerRef.current?.focus();
      }
    }
    document.addEventListener("pointerdown", closeOnOutside);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const focusDate =
      selected !== null &&
      selected[0] === view.year &&
      selected[1] === view.month
        ? isoDate(...selected)
        : isoDate(view.year, view.month, 1);
    window.requestAnimationFrame(() => dayRefs.current[focusDate]?.focus());
  }, [open, selected, selectedDate, view.month, view.year]);

  function shiftMonth(delta: number) {
    const year = commitYearText();
    const next = new Date(Date.UTC(year, view.month + delta, 1));
    setView({ year: next.getUTCFullYear(), month: next.getUTCMonth() });
    setYearText(String(next.getUTCFullYear()));
  }

  function selectMonth(month: number) {
    const year = commitYearText();
    setView({ year, month });
  }

  function selectYear(text: string) {
    if (/^\d{0,4}$/.test(text)) setYearText(text);
  }

  function commitYearText(): number {
    if (/^\d{4}$/.test(yearText)) {
      const year = Number(yearText);
      if (year >= 1 && year <= 9999) {
        setView((current) => ({ ...current, year }));
        return year;
      }
    }
    setYearText(String(view.year));
    return view.year;
  }

  function selectDay(day: number) {
    onChange(isoDate(commitYearText(), view.month, day));
    setOpen(false);
  }

  function moveDay(day: number, delta: number) {
    const next = new Date(Date.UTC(commitYearText(), view.month, day + delta));
    const nextValue = isoDate(
      next.getUTCFullYear(),
      next.getUTCMonth(),
      next.getUTCDate(),
    );
    setView({ year: next.getUTCFullYear(), month: next.getUTCMonth() });
    onChange(nextValue);
    window.requestAnimationFrame(() => dayRefs.current[nextValue]?.focus());
  }

  return (
    <div className="date-picker" ref={pickerRef}>
      <button
        ref={triggerRef}
        type="button"
        className="date-picker-trigger"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={popoverId}
        onClick={() => {
          if (!open && selected !== null)
            setView({ year: selected[0], month: selected[1] });
          if (!open && selected !== null) setYearText(String(selected[0]));
          setOpen((current) => !current);
        }}
      >
        {selected === null
          ? "Selecionar data"
          : `${selected[2].toString().padStart(2, "0")}/${(selected[1] + 1)
              .toString()
              .padStart(2, "0")}/${selected[0]}`}
      </button>
      {open ? (
        <div
          id={popoverId}
          className="date-picker-popover"
          role="dialog"
          aria-label="Selecionar data"
        >
          <div className="date-picker-header">
            <button
              type="button"
              aria-label="Mês anterior"
              onClick={() => shiftMonth(-1)}
            >
              ‹
            </button>
            <div
              className="date-picker-navigation"
              aria-label="Navegação do calendário"
            >
              <label>
                <span className="visually-hidden">Mês</span>
                <select
                  aria-label="Mês"
                  value={view.month}
                  onChange={(event) => selectMonth(Number(event.target.value))}
                >
                  {Array.from({ length: 12 }, (_, month) => (
                    <option value={month} key={month}>
                      {new Intl.DateTimeFormat("pt-BR", {
                        month: "long",
                        timeZone: "UTC",
                      }).format(new Date(Date.UTC(2000, month, 1)))}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span className="visually-hidden">Ano</span>
                <input
                  aria-label="Ano"
                  inputMode="numeric"
                  min="1"
                  max="9999"
                  value={yearText}
                  onChange={(event) => selectYear(event.target.value)}
                  onBlur={commitYearText}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      commitYearText();
                    }
                  }}
                />
              </label>
            </div>
            <button
              type="button"
              aria-label="Próximo mês"
              onClick={() => shiftMonth(1)}
            >
              ›
            </button>
          </div>
          <div className="date-picker-weekdays" aria-hidden="true">
            {["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"].map((day) => (
              <span key={day}>{day}</span>
            ))}
          </div>
          <div className="date-picker-grid" role="grid" aria-label={monthLabel}>
            {Array.from({ length: firstWeekday }, (_, index) => (
              <span key={`empty-${index}`} aria-hidden="true" />
            ))}
            {days.map((day) => {
              const current = isoDate(view.year, view.month, day);
              return (
                <button
                  type="button"
                  role="gridcell"
                  key={current}
                  aria-label={current}
                  aria-selected={current === selectedDate}
                  tabIndex={
                    current ===
                    (selectedDate ?? isoDate(view.year, view.month, 1))
                      ? 0
                      : -1
                  }
                  ref={(element) => {
                    dayRefs.current[current] = element;
                  }}
                  className={current === selectedDate ? "selected" : undefined}
                  onClick={() => selectDay(day)}
                  onKeyDown={(event) => {
                    const offsets: Record<string, number> = {
                      ArrowLeft: -1,
                      ArrowRight: 1,
                      ArrowUp: -7,
                      ArrowDown: 7,
                    };
                    const offset = offsets[event.key];
                    if (offset !== undefined) {
                      event.preventDefault();
                      moveDay(day, offset);
                    }
                  }}
                >
                  {day}
                </button>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function AgentButton({
  agent,
  active,
  onSelect,
}: {
  agent: ProvisionalAgent;
  active: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      className={active ? "agent-tab active" : "agent-tab"}
      type="button"
      onClick={onSelect}
    >
      <AgentSprite
        agentId={agent.id}
        spriteKey={agent.spriteKey}
        name={agent.name}
      />
      <span>{agent.name}</span>
    </button>
  );
}

function ConfirmDialog({
  title,
  description,
  confirmLabel,
  onCancel,
  onConfirm,
}: {
  title: string;
  description: string;
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const titleId = useId();
  const descriptionId = useId();
  const cancelRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const previouslyFocused =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    cancelRef.current?.focus();
    return () => previouslyFocused?.focus();
  }, []);

  function handleKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
      return;
    }
    if (event.key !== "Tab") return;
    if (event.shiftKey && document.activeElement === cancelRef.current) {
      event.preventDefault();
      confirmRef.current?.focus();
    } else if (
      !event.shiftKey &&
      document.activeElement === confirmRef.current
    ) {
      event.preventDefault();
      cancelRef.current?.focus();
    }
  }

  return (
    <div className="aip-modal-backdrop">
      <div
        className="aip-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        onKeyDown={handleKeyDown}
      >
        <h2 id={titleId}>{title}</h2>
        <p id={descriptionId}>{description}</p>
        <div className="aip-modal-actions">
          <button ref={cancelRef} type="button" onClick={onCancel}>
            Cancelar
          </button>
          <button
            ref={confirmRef}
            type="button"
            className="danger-action"
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

type ModelPickerOption = {
  ref: string | null;
  label: string;
  detail: string;
  searchText: string;
  unavailable?: boolean;
};

const modelPickerStateCopy: Record<
  ProviderSnapshot["state"],
  { label: string; detail: string }
> = {
  checking: {
    label: "Verificando modelos locais…",
    detail: "Aguarde a resposta do Ollama.",
  },
  available: {
    label: "Nenhum modelo local disponível",
    detail: "Instale ou atualize um modelo local para continuar.",
  },
  empty: {
    label: "Nenhum modelo local disponível",
    detail: "O Ollama está ativo, mas não há modelos instalados.",
  },
  unavailable: {
    label: "Ollama indisponível",
    detail: "Inicie o Ollama para consultar os modelos locais.",
  },
  malformed: {
    label: "Resposta inválida do Ollama",
    detail: "O provedor devolveu dados que não puderam ser usados.",
  },
  timeout: {
    label: "O Ollama não respondeu a tempo",
    detail: "Tente atualizar os modelos novamente.",
  },
};

function modelSizeLabel(size: number): string | null {
  if (!Number.isFinite(size) || size <= 0) return null;
  const gigabytes = size / 1_000_000_000;
  return `${gigabytes >= 1 ? gigabytes.toFixed(1) : (size / 1_000_000).toFixed(0)} ${gigabytes >= 1 ? "GB" : "MB"}`;
}

function modelOption(model: OllamaModel): ModelPickerOption {
  const capabilities = model.capabilities ?? [];
  const metadata = [
    "Ollama",
    model.parameterSize,
    model.quantization,
    model.family,
    modelSizeLabel(model.size),
    capabilities.length > 0 ? capabilities.join(", ") : null,
  ].filter(
    (value): value is string => typeof value === "string" && value.length > 0,
  );
  return {
    ref: model.ref,
    label: model.displayName,
    detail: metadata.join(" · "),
    searchText: [
      model.displayName,
      model.ref,
      model.providerModelId,
      ...metadata,
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function modelPickerOptionId(
  listboxId: string,
  option: ModelPickerOption,
): string {
  return `${listboxId}-option-${option.ref === null ? "default" : encodeURIComponent(option.ref)}`;
}

export function ModelPicker({
  label,
  ariaLabel,
  models,
  value,
  providerState,
  defaultOption,
  statusText,
  disabled = false,
  onSelect,
}: {
  label: string;
  ariaLabel?: string;
  models: OllamaModel[];
  value: string | null;
  providerState: ProviderSnapshot["state"];
  defaultOption?: { label: string; detail: string };
  statusText?: string;
  disabled?: boolean;
  onSelect: (modelRef: string | null) => void | Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [selecting, setSelecting] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const listboxId = useId();

  const options: ModelPickerOption[] = [
    ...(defaultOption
      ? [
          {
            ref: null,
            label: defaultOption.label,
            detail: defaultOption.detail,
            searchText:
              `${defaultOption.label} ${defaultOption.detail}`.toLowerCase(),
          },
        ]
      : []),
    ...models.map(modelOption),
    ...(value !== null && !models.some((model) => model.ref === value)
      ? [
          {
            ref: value,
            label: value,
            detail: "Indisponível · selecione outro modelo",
            searchText: `${value} indisponível`.toLowerCase(),
            unavailable: true,
          },
        ]
      : []),
  ];
  const normalizedQuery = query.trim().toLowerCase();
  const filteredOptions = normalizedQuery
    ? options.filter((option) => option.searchText.includes(normalizedQuery))
    : options;
  const selectedIndex = filteredOptions.findIndex(
    (option) => option.ref === value,
  );
  const activeOption = filteredOptions[activeIndex];
  const activeOptionId =
    activeOption === undefined
      ? undefined
      : modelPickerOptionId(listboxId, activeOption);
  const selectedOption = options.find((option) => option.ref === value);
  const stateCopy = modelPickerStateCopy[providerState];
  const triggerLabel =
    selectedOption?.label ??
    (models.length > 0 ? "Selecione um modelo local" : stateCopy.label);
  const triggerDetail = selectedOption?.detail ?? stateCopy.detail;

  useEffect(() => {
    if (open) searchRef.current?.focus();
  }, [open]);
  useEffect(() => {
    if (!open) return;
    setActiveIndex(query.trim() ? 0 : selectedIndex >= 0 ? selectedIndex : 0);
  }, [open, query, selectedIndex]);
  useEffect(() => {
    if (!open) return;
    function closeFromOutside(event: PointerEvent) {
      if (
        rootRef.current !== null &&
        event.target instanceof Node &&
        !rootRef.current.contains(event.target)
      ) {
        setOpen(false);
        setQuery("");
      }
    }
    document.addEventListener("pointerdown", closeFromOutside);
    return () => document.removeEventListener("pointerdown", closeFromOutside);
  }, [open]);

  function closePicker() {
    setOpen(false);
    setQuery("");
    triggerRef.current?.focus();
  }
  function openPicker() {
    if (disabled || selecting) return;
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0);
    setOpen(true);
  }
  async function selectOption(option: ModelPickerOption) {
    if (selecting || option.unavailable) return;
    setSelecting(true);
    try {
      await onSelect(option.ref);
      setOpen(false);
      setQuery("");
      triggerRef.current?.focus();
    } finally {
      setSelecting(false);
    }
  }
  function handleSearchKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      closePicker();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((index) =>
        filteredOptions.length === 0
          ? 0
          : Math.min(index + 1, filteredOptions.length - 1),
      );
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((index) => Math.max(index - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = filteredOptions[activeIndex];
      if (option !== undefined) void selectOption(option);
    }
  }

  return (
    <div
      ref={rootRef}
      className="model-picker-field"
      data-provider-state={providerState}
    >
      <span className="model-picker-label">{label}</span>
      <div className="model-picker">
        <button
          ref={triggerRef}
          type="button"
          className="model-picker-trigger"
          aria-label={ariaLabel ?? label}
          aria-haspopup="listbox"
          aria-expanded={open}
          aria-controls={listboxId}
          disabled={disabled || selecting}
          onClick={() => (open ? closePicker() : openPicker())}
          onKeyDown={(event) => {
            if (event.key === "Escape" && open) {
              event.preventDefault();
              closePicker();
            } else if (
              event.key === "Enter" ||
              event.key === " " ||
              event.key === "ArrowDown"
            ) {
              event.preventDefault();
              openPicker();
            }
          }}
        >
          <span className="model-picker-trigger-copy">
            <strong>{triggerLabel}</strong>
            <small>{triggerDetail}</small>
          </span>
          <span aria-hidden="true">⌄</span>
        </button>
        {open ? (
          <div className="model-picker-popover">
            <input
              ref={searchRef}
              type="search"
              value={query}
              aria-label={`Buscar em ${label.toLowerCase()}`}
              aria-controls={listboxId}
              aria-activedescendant={activeOptionId}
              placeholder="Buscar por nome, ref. ou metadados"
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={handleSearchKeyDown}
            />
            <div
              id={listboxId}
              className="model-picker-options"
              role="listbox"
              aria-label={label}
            >
              {filteredOptions.length > 0 ? (
                filteredOptions.map((option, index) => (
                  <div
                    key={option.ref ?? "default"}
                    id={modelPickerOptionId(listboxId, option)}
                    role="option"
                    aria-selected={option.ref === value}
                    aria-disabled={option.unavailable}
                    data-active={index === activeIndex}
                    className={
                      index === activeIndex
                        ? "model-picker-option active"
                        : "model-picker-option"
                    }
                    onMouseEnter={() => setActiveIndex(index)}
                    onClick={() => void selectOption(option)}
                  >
                    <strong>
                      {option.label}
                      {option.unavailable ? " · indisponível" : ""}
                    </strong>
                    <small>{option.detail}</small>
                  </div>
                ))
              ) : (
                <p className="model-picker-empty">
                  Nenhum modelo corresponde à busca.
                </p>
              )}
            </div>
          </div>
        ) : null}
      </div>
      <small className="model-picker-status">
        {providerState !== "available"
          ? `${stateCopy.label} · ${statusText ?? stateCopy.detail}`
          : (statusText ??
            (models.length > 0
              ? "Escolha uma opção local."
              : stateCopy.detail))}
      </small>
    </div>
  );
}

function MessageItem({
  message,
  onRegenerate,
  onEdit,
  variants = [],
  onSelectVariant,
  retrying,
  models,
}: {
  message: ConversationMessage;
  onRegenerate: (message: ConversationMessage, modelRef?: string) => void;
  onEdit: (message: ConversationMessage, content: string) => void;
  variants?: Array<{ id: string; branchId: string; active: boolean }>;
  onSelectVariant?: (id: string) => void;
  retrying: boolean;
  models: OllamaModel[];
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(message.content);
  const [retryMenuOpen, setRetryMenuOpen] = useState(false);
  const [advancedRetry, setAdvancedRetry] = useState(false);
  const [retryModel, setRetryModel] = useState(message.modelRef ?? "");
  const retryable =
    message.author === "agent" &&
    message.status !== "pending" &&
    message.status !== "streaming";
  const pendingAssistant =
    message.author === "agent" &&
    (message.status === "pending" || message.status === "streaming");
  return (
    <article
      className={`chat-message ${message.author}`}
      data-status={message.status}
    >
      <div className="message-heading">
        <strong>{message.author === "user" ? "Você" : "Agente"}</strong>
        <span>{messageStatusCopy(message)}</span>
      </div>
      {editing ? (
        <div className="message-editor">
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
          />
          <button
            type="button"
            onClick={() => {
              setEditing(false);
              setDraft(message.content);
            }}
          >
            Cancelar
          </button>
          <button
            type="button"
            disabled={!draft.trim()}
            onClick={() => {
              onEdit(message, draft.trim());
              setEditing(false);
            }}
          >
            Salvar e enviar
          </button>
        </div>
      ) : message.content ? (
        <p>{message.content}</p>
      ) : null}
      {!pendingAssistant ? (
        <div className="message-actions">
          <button
            type="button"
            onClick={() => void navigator.clipboard?.writeText(message.content)}
          >
            Copiar
          </button>
          {message.author === "user" ? (
            <button type="button" onClick={() => setEditing(true)}>
              Editar
            </button>
          ) : null}
          {retryable ? (
            <>
              <button
                type="button"
                disabled={retrying}
                onClick={() => onRegenerate(message)}
              >
                Tentar novamente
              </button>
              <button
                type="button"
                aria-label="Opções de tentativa"
                disabled={retrying}
                onClick={() => setRetryMenuOpen(!retryMenuOpen)}
              >
                ⌄
              </button>
              {retryMenuOpen ? (
                <div className="retry-menu">
                  <button type="button" onClick={() => onRegenerate(message)}>
                    Tentar novamente
                  </button>
                  <button
                    type="button"
                    onClick={() => setAdvancedRetry(!advancedRetry)}
                  >
                    Avançado
                  </button>
                  {advancedRetry ? (
                    <div>
                      <ModelPicker
                        label="Modelo para nova tentativa"
                        ariaLabel="Modelo para nova tentativa"
                        models={models}
                        value={retryModel || null}
                        providerState="available"
                        disabled={retrying}
                        statusText={`Modelo usado: ${message.modelRef ?? "indisponível"}`}
                        onSelect={(modelRef) => {
                          if (modelRef !== null) setRetryModel(modelRef);
                        }}
                      />
                      <button
                        type="button"
                        disabled={retrying || !retryModel}
                        onClick={() => onRegenerate(message, retryModel)}
                      >
                        Tentar com este modelo
                      </button>
                    </div>
                  ) : null}
                </div>
              ) : null}
            </>
          ) : null}
          {message.author === "agent" && message.modelRef ? (
            <details>
              <summary>Modelo</summary>
              <span>{message.modelRef}</span>
            </details>
          ) : null}
          {variants.length > 1 ? (
            <span className="turn-variants">
              <button
                type="button"
                disabled={variants.findIndex((variant) => variant.active) <= 0}
                onClick={() => {
                  const index = variants.findIndex((variant) => variant.active);
                  if (index > 0)
                    onSelectVariant?.(variants[index - 1]!.branchId);
                }}
              >
                ‹
              </button>
              <span>
                {variants.findIndex((variant) => variant.active) + 1}/
                {variants.length}
              </span>
              <button
                type="button"
                disabled={
                  variants.findIndex((variant) => variant.active) >=
                  variants.length - 1
                }
                onClick={() => {
                  const index = variants.findIndex((variant) => variant.active);
                  if (index >= 0 && index < variants.length - 1)
                    onSelectVariant?.(variants[index + 1]!.branchId);
                }}
              >
                ›
              </button>
            </span>
          ) : null}
        </div>
      ) : null}
    </article>
  );
}

export function ConversationSurface({
  agentId,
  temporary,
  onToggleTemporary,
  refreshRevision = 0,
}: {
  agentId: string;
  temporary: boolean;
  onToggleTemporary?: () => void;
  refreshRevision?: number;
}) {
  const { phase, error, load } = usePhaseOne(agentId, temporary);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [showLegacyBranchPicker] = useState(false);
  const [cancellingRequestId, setCancellingRequestId] = useState<string | null>(
    null,
  );
  const [retryingMessageId, setRetryingMessageId] = useState<string | null>(
    null,
  );
  const historyRef = useRef<HTMLDivElement>(null);
  const conversationIdRef = useRef<string | null>(null);
  const followsBottomRef = useRef(true);

  useEffect(() => {
    if (refreshRevision > 0) void load();
  }, [load, refreshRevision]);

  useLayoutEffect(() => {
    const history = historyRef.current;
    const conversationId = phase?.conversation.id;
    if (history === null || conversationId === undefined) return;
    const conversationChanged = conversationIdRef.current !== conversationId;
    if (
      shouldScrollConversationToBottom(
        conversationChanged,
        followsBottomRef.current,
      )
    ) {
      history.scrollTop = history.scrollHeight;
      followsBottomRef.current = true;
    }
    conversationIdRef.current = conversationId;
  }, [phase?.conversation.id, phase?.messages]);

  useEffect(() => {
    const activeRequest = phase
      ? requestForAgent(phase.queue, phase.agent.id)
      : null;
    if (
      cancellingRequestId !== null &&
      activeRequest?.requestId !== cancellingRequestId
    ) {
      setCancellingRequestId(null);
    }
  }, [cancellingRequestId, phase]);

  if (error) {
    return (
      <section className="conversation-empty" role="alert">
        <p>Não foi possível carregar a conversa local.</p>
        <button type="button" onClick={() => void load()}>
          Reiniciar runtime
        </button>
      </section>
    );
  }
  if (phase === null)
    return (
      <section className="conversation-empty">Carregando conversa…</section>
    );

  const currentPhase = phase;
  const request = requestForAgent(phase.queue, phase.agent.id);
  const blocked = blockedSendCopy(phase.sendBlockedCode);
  const providerRecovery = providerRecoveryCopy(phase);
  const modelsAvailable = phase.provider.models.length > 0;
  const canSend = canSendConversationMessage(currentPhase);
  const canDraft = canDraftConversationMessage(currentPhase);

  async function send() {
    const content = draft.trim();
    if (!content || busy || !canSend) return;
    followsBottomRef.current = true;
    setBusy(true);
    try {
      await invoke(
        temporary
          ? "send_temporary_phase_one_message"
          : "send_phase_one_message",
        temporary
          ? { agentId: currentPhase.agent.id, content }
          : {
              agentId: currentPhase.agent.id,
              conversationId: currentPhase.conversation.id,
              content,
            },
      );
      setDraft("");
      void load();
    } finally {
      setBusy(false);
    }
  }

  async function refreshModels() {
    await invoke("refresh_ollama_models").catch(() => null);
    await load();
  }

  async function cancelCurrentRequest() {
    if (
      request === null ||
      !canRequestCancellation(request, cancellingRequestId)
    )
      return;
    setCancellingRequestId(request.requestId);
    try {
      await invoke("cancel_phase_one_generation", {
        requestId: request.requestId,
      });
      void load();
    } catch {
      setCancellingRequestId(null);
    }
  }

  async function regenerate(message: ConversationMessage, modelRef?: string) {
    if (
      temporary ||
      message.status === "pending" ||
      message.status === "streaming"
    )
      return;
    setRetryingMessageId(message.id);
    try {
      await invoke("regenerate_phase_one_message", {
        agentId: currentPhase.agent.id,
        conversationId: currentPhase.conversation.id,
        assistantMessageId: message.id,
        modelRef,
        requestId: crypto.randomUUID(),
      });
      void load();
    } finally {
      setRetryingMessageId(null);
    }
  }

  async function edit(message: ConversationMessage, content: string) {
    if (temporary) return;
    await invoke("edit_phase_one_message", {
      agentId: currentPhase.agent.id,
      conversationId: currentPhase.conversation.id,
      userMessageId: message.id,
      content,
    });
    void load();
  }

  return (
    <section
      className="conversation-surface"
      aria-label={`Conversa com ${phase.agent.name}`}
    >
      <header className="conversation-header">
        <div>
          <p className="eyebrow">
            {temporary ? "Conversa temporária" : "Conversa"}
          </p>
          <h1>{phase.agent.name}</h1>
          <span className="conversation-title">{phase.conversation.title}</span>
          <span className={`provider-state ${phase.provider.state}`}>
            {providerStatusCopy(phase)}
          </span>
        </div>
        <div className="conversation-controls">
          {onToggleTemporary ? (
            <button
              className={
                temporary ? "temporary-control active" : "temporary-control"
              }
              type="button"
              aria-pressed={temporary}
              onClick={onToggleTemporary}
            >
              {temporary
                ? "Encerrar conversa temporária"
                : "Iniciar conversa temporária"}
            </button>
          ) : null}
          {temporary ? (
            <p className="temporary-disclosure" role="status">
              Temporária ativa: mensagens e contexto ficam apenas na memória e
              são apagados ao encerrar.
            </p>
          ) : null}
          {providerRecovery ? (
            <p className="provider-recovery" role="status">
              {providerRecovery}
            </p>
          ) : null}
          <details className="model-advanced-controls">
            <summary>Opções do modelo</summary>
            <div>
              <button type="button" onClick={() => void refreshModels()}>
                Atualizar modelos locais
              </button>
              <label>
                <span>Manter modelo carregado</span>
                <select
                  value={phase.keepAliveMinutes}
                  onChange={(event) =>
                    void invoke("update_keep_alive", {
                      agentId: currentPhase.agent.id,
                      minutes: Number(event.target.value),
                    }).then(load)
                  }
                >
                  <option value={0}>Descarregar imediatamente</option>
                  <option value={5}>5 minutos</option>
                  <option value={15}>15 minutos</option>
                  <option value={30}>30 minutos</option>
                  <option value={60}>60 minutos</option>
                  <option value={120}>120 minutos</option>
                </select>
              </label>
            </div>
          </details>
        </div>
      </header>

      <div
        className="message-history"
        ref={historyRef}
        aria-live="polite"
        onScroll={() => {
          const history = historyRef.current;
          if (history !== null) {
            followsBottomRef.current = isNearConversationBottom(history);
          }
        }}
      >
        {showLegacyBranchPicker &&
        !temporary &&
        currentPhase.branches.length > 1 ? (
          <label className="branch-picker">
            <span>Alternativa</span>
            <select
              value={currentPhase.activeBranchId ?? ""}
              onChange={(event) =>
                void invoke("set_active_conversation_branch", {
                  agentId: currentPhase.agent.id,
                  conversationId: currentPhase.conversation.id,
                  branchId: event.target.value,
                }).then(load)
              }
            >
              {currentPhase.branches.map((branch, index) => (
                <option value={branch.id} key={branch.id}>
                  ‹ {index + 1}/{currentPhase.branches.length} ›
                </option>
              ))}
            </select>
          </label>
        ) : null}
        {phase.messages.length === 0 ? (
          <div className="history-placeholder">
            <strong>Esta conversa ainda está vazia.</strong>
            <span>
              Escolha um modelo local e envie uma mensagem para começar.
            </span>
          </div>
        ) : (
          phase.messages.map((message) => (
            <MessageItem
              key={message.id}
              message={message}
              onRegenerate={(message, modelRef) =>
                void regenerate(message, modelRef)
              }
              onEdit={(message, content) => void edit(message, content)}
              retrying={retryingMessageId === message.id}
              models={currentPhase.provider.models}
              variants={
                message.author === "agent"
                  ? currentPhase.turnVariants
                      .filter(
                        (variant) =>
                          variant.turnGroupId === message.turnGroupId,
                      )
                      .map((variant) => ({
                        id: variant.assistantMessageId,
                        branchId: variant.branchId,
                        active: variant.assistantMessageId === message.id,
                      }))
                  : []
              }
              onSelectVariant={(branchId) =>
                void invoke("set_active_conversation_branch", {
                  agentId: currentPhase.agent.id,
                  conversationId: currentPhase.conversation.id,
                  branchId,
                }).then(load)
              }
            />
          ))
        )}
      </div>

      <footer className="composer">
        {request !== null ? (
          <div className="queue-banner">
            <span>
              {request.active
                ? request.cancellationRequested
                  ? "Cancelando resposta…"
                  : "Gerando resposta…"
                : "Aguardando processamento…"}
            </span>
            <button
              type="button"
              disabled={!canRequestCancellation(request, cancellingRequestId)}
              onClick={() => void cancelCurrentRequest()}
            >
              {request.cancellationRequested ||
              cancellingRequestId === request.requestId
                ? "Cancelando…"
                : "Cancelar"}
            </button>
          </div>
        ) : null}
        <textarea
          value={draft}
          maxLength={16_384}
          placeholder={blocked ?? `Escreva para ${phase.agent.name}`}
          disabled={!canDraft || busy}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void send();
            }
          }}
        />
        <div className="composer-footer">
          <div className="composer-actions">
            <div className="conversation-model-selector">
              <ModelPicker
                label="Modelo desta conversa"
                ariaLabel="Modelo desta conversa"
                models={phase.provider.models}
                value={phase.modelOverrideRef}
                providerState={phase.provider.state}
                disabled={!modelsAvailable}
                defaultOption={{
                  label: `Usar padrão de ${phase.agent.name}`,
                  detail: phase.defaultModelRef ?? "indisponível",
                }}
                statusText={
                  phase.selectedModelRef
                    ? `Em uso: ${phase.selectedModelRef}${phase.effectiveModelSource === "agent_default" ? " · padrão do agente" : " · substituição desta conversa"}`
                    : "Nenhum modelo local disponível"
                }
                onSelect={async (modelRef) => {
                  const command = temporary
                    ? "set_temporary_phase_one_model"
                    : "set_conversation_model_override";
                  const argumentsForCommand = temporary
                    ? { agentId: currentPhase.agent.id, modelRef }
                    : conversationOverrideArguments(
                        currentPhase.agent.id,
                        currentPhase.conversation.id,
                        modelRef ?? "",
                      );
                  await invoke(command, argumentsForCommand);
                  await load();
                }}
              />
              <span>
                {blocked ?? "Enter envia · Shift+Enter cria uma linha"}
              </span>
            </div>
          </div>
          <button
            type="button"
            disabled={!canSend || !draft.trim() || busy}
            onClick={() => void send()}
          >
            Enviar
          </button>
        </div>
      </footer>
    </section>
  );
}

export function ConversationDraftSurface({
  agentId,
  onCreated,
  onPersisted,
}: {
  agentId: string;
  onCreated?: () => void;
  onPersisted: () => void;
}) {
  const { phase, error, load } = usePhaseOne(agentId, false);
  const [title, setTitle] = useState("");
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [persistedConversationId, setPersistedConversationId] = useState<
    string | null
  >(null);
  const [cancellingRequestId, setCancellingRequestId] = useState<string | null>(
    null,
  );

  if (error) {
    return (
      <section className="conversation-empty" role="alert">
        <p>Não foi possível carregar os dados locais para este rascunho.</p>
        <button type="button" onClick={() => void load()}>
          Tentar novamente
        </button>
      </section>
    );
  }
  if (phase === null)
    return (
      <section className="conversation-empty">Carregando rascunho…</section>
    );

  const currentPhase: PhaseOneState = phase;
  const request = requestForAgent(currentPhase.queue, agentId);
  const blocked = blockedSendCopy(currentPhase.sendBlockedCode);
  const canSend = canSendConversationMessage(currentPhase);
  const canDraft = canDraftConversationMessage(currentPhase);

  async function cancelCurrentRequest() {
    if (
      request === null ||
      !canRequestCancellation(request, cancellingRequestId)
    )
      return;
    setCancellingRequestId(request.requestId);
    try {
      await invoke("cancel_phase_one_generation", {
        requestId: request.requestId,
      });
      void load();
    } catch {
      setCancellingRequestId(null);
    }
  }

  async function persist(sendContent: string | null) {
    const trimmedTitle = title.trim();
    if (sendContent === null && !trimmedTitle) {
      setErrorMessage("Informe um nome para salvar a conversa.");
      return;
    }
    if (busy || (sendContent !== null && (!sendContent || !canSend))) return;
    setBusy(true);
    setErrorMessage(null);
    let conversationId = persistedConversationId;
    try {
      if (conversationId === null) {
        const persistedTitle = (trimmedTitle || "Nova conversa").slice(0, 160);
        const created = await invoke<PhaseOneConversation>(
          "create_agent_conversation",
          { agentId, title: persistedTitle },
        );
        if (!created?.id) throw new Error("conversation_create_failed");
        conversationId = created.id;
        setPersistedConversationId(conversationId);
      }
      await invoke("set_active_agent_conversation", {
        agentId,
        conversationId,
      });
      if (persistedConversationId === null) onCreated?.();
      if (sendContent !== null) {
        await invoke("send_phase_one_message", {
          agentId,
          conversationId,
          content: sendContent,
        });
        setDraft("");
      }
      onPersisted();
    } catch {
      setErrorMessage(
        sendContent !== null
          ? "Não foi possível enviar. O rascunho foi preservado; tente novamente."
          : "Não foi possível salvar a conversa local.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section
      className="conversation-surface conversation-draft-surface"
      aria-label={`Nova conversa com ${currentPhase.agent.name}`}
    >
      <header className="conversation-header">
        <div>
          <p className="eyebrow">Rascunho local</p>
          <h1>{currentPhase.agent.name}</h1>
          <span className="conversation-title">
            Ainda não foi salvo no histórico
          </span>
          <span className={`provider-state ${currentPhase.provider.state}`}>
            {providerStatusCopy(currentPhase)}
          </span>
        </div>
        <div className="conversation-controls draft-controls">
          <label className="draft-title-field">
            <span>Nome da conversa (opcional)</span>
            <input
              value={title}
              maxLength={160}
              placeholder="Nova conversa"
              disabled={busy}
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>
          <button
            type="button"
            disabled={busy || !title.trim()}
            onClick={() => void persist(null)}
          >
            Salvar nome
          </button>
        </div>
      </header>

      <div className="message-history" aria-live="polite">
        <div className="history-placeholder">
          <strong>Rascunho ainda não persistido.</strong>
          <span>Salve um nome ou envie a primeira mensagem para começar.</span>
        </div>
      </div>

      <footer className="composer">
        {request !== null ? (
          <div className="queue-banner">
            <span>
              {request.active
                ? request.cancellationRequested
                  ? "Cancelando resposta…"
                  : "Gerando resposta…"
                : "Aguardando processamento…"}
            </span>
            <button
              type="button"
              disabled={!canRequestCancellation(request, cancellingRequestId)}
              onClick={() => void cancelCurrentRequest()}
            >
              {request.cancellationRequested ||
              cancellingRequestId === request.requestId
                ? "Cancelando…"
                : "Cancelar"}
            </button>
          </div>
        ) : null}
        <textarea
          value={draft}
          maxLength={16_384}
          placeholder={blocked ?? `Escreva para ${currentPhase.agent.name}`}
          disabled={!canDraft || busy}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void persist(draft.trim());
            }
          }}
        />
        <div className="composer-footer">
          <span>
            {blocked ?? "Enter salva e envia · Shift+Enter cria uma linha"}
          </span>
          <button
            type="button"
            disabled={!canSend || !draft.trim() || busy}
            onClick={() => void persist(draft.trim())}
          >
            Enviar
          </button>
        </div>
        {errorMessage ? (
          <div className="draft-error" role="alert">
            <span>{errorMessage}</span>
            {persistedConversationId !== null && draft.trim() ? (
              <button
                type="button"
                disabled={busy}
                onClick={() => void persist(draft.trim())}
              >
                Tentar novamente
              </button>
            ) : null}
          </div>
        ) : null}
      </footer>
    </section>
  );
}

function ProfileCanonicalSelect({
  field,
  label,
  value,
  onChange,
}: {
  field: ProfileCanonicalField;
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  const current = localizedCanonicalValue(field, value);
  const options = profileCanonicalOptions[field];
  return (
    <label>
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        {!options.some((option) => option.value === value) ? (
          <option value={value}>{current.primary}</option>
        ) : null}
        {options.map((option) => (
          <option value={option.value} key={option.value}>
            {option.primary}
            {option.secondary ? ` (${option.secondary})` : ""}
          </option>
        ))}
      </select>
    </label>
  );
}

function ProfileFields({
  draft,
  onChange,
  fictiveAgeText,
  onFictiveAgeTextChange,
}: {
  draft: ProvisionalAgent;
  onChange: (update: ProfileDraftUpdater) => void;
  fictiveAgeText?: string;
  onFictiveAgeTextChange?: (value: string) => void;
}) {
  const hasCanonicalPronouns = profileCanonicalOptions.pronouns.some(
    (option) => option.value === draft.pronouns,
  );
  const customPronouns = !hasCanonicalPronouns;
  const humanCompatible = isHumanCompatibleSpecies(draft.species);
  return (
    <>
      <label>
        Nome
        <input
          value={draft.name}
          onChange={(event) =>
            onChange((current) => ({ ...current, name: event.target.value }))
          }
        />
      </label>
      <label>
        Data de aniversário
        <DatePicker
          value={draft.birthday}
          onChange={(birthday) =>
            onChange((current) => ({ ...current, birthday }))
          }
        />
      </label>
      <label>
        Idade fictícia
        <input
          type="text"
          inputMode="numeric"
          min="0"
          max="10000"
          value={fictiveAgeText ?? String(draft.fictiveAge)}
          onChange={(event) => {
            const text = event.target.value;
            onFictiveAgeTextChange?.(text);
            onChange((current) => ({ ...current, fictiveAge: Number(text) }));
          }}
        />
      </label>
      <ProfileCanonicalSelect
        field="ageCategory"
        label="Categoria de idade"
        value={draft.ageCategory}
        onChange={(ageCategory) =>
          onChange((current) => ({ ...current, ageCategory }))
        }
      />
      <ProfileCanonicalSelect
        field="species"
        label="Tipo de identidade"
        value={draft.species}
        onChange={(species) => onChange((current) => ({ ...current, species }))}
      />
      <ProfileCanonicalSelect
        field="pronouns"
        label="Pronomes"
        value={customPronouns ? "custom" : draft.pronouns}
        onChange={(pronouns) =>
          onChange((current) => ({
            ...current,
            pronouns:
              pronouns === "custom"
                ? customPronouns
                  ? current.pronouns
                  : ""
                : pronouns,
          }))
        }
      />
      {customPronouns ? (
        <label>
          Pronomes personalizados
          <input
            value={draft.pronouns}
            placeholder="Ex.: elu/delu"
            onChange={(event) =>
              onChange((current) => ({
                ...current,
                pronouns: event.target.value,
              }))
            }
          />
        </label>
      ) : null}
      {humanCompatible ? (
        <>
          <label>
            Gênero (opcional)
            <input
              value={draft.gender ?? ""}
              onChange={(event) =>
                onChange((current) => ({
                  ...current,
                  gender: event.target.value || null,
                }))
              }
            />
          </label>
          <label>
            Sexualidade (opcional)
            <input
              value={draft.sexuality ?? ""}
              onChange={(event) =>
                onChange((current) => ({
                  ...current,
                  sexuality: event.target.value || null,
                }))
              }
            />
          </label>
        </>
      ) : null}
      <label className="profile-description-field">
        Descrição
        <textarea
          rows={4}
          value={draft.personalitySummary}
          onChange={(event) =>
            onChange((current) => ({
              ...current,
              personalitySummary: event.target.value,
            }))
          }
        />
      </label>
      <fieldset className="trait-controls">
        <legend>Traços iniciais</legend>
        <input
          type="hidden"
          value={draft.traitsJson}
          onChange={(event) =>
            onChange((current) => ({
              ...current,
              traitsJson: event.target.value,
            }))
          }
        />
        <div className="trait-grid">
          {initialTraits.map(([key, label]) => (
            <label className="trait-card" key={key}>
              <span>{label}</span>
              <input
                type="number"
                min="0"
                max="100"
                step="1"
                value={traitValues(draft.traitsJson)[key] ?? 50}
                onChange={(event) =>
                  onChange((current) =>
                    updateInitialTrait(
                      current,
                      key,
                      Number(event.target.value),
                    ),
                  )
                }
              />
            </label>
          ))}
        </div>
      </fieldset>
    </>
  );
}

export function ProfileForm({
  agent,
  done,
}: {
  agent: ProvisionalAgent;
  done: () => void;
}) {
  const [draft, setDraft] = useState(agent);
  const [persisted, setPersisted] = useState(agent);
  const [fictiveAgeText, setFictiveAgeText] = useState(
    String(agent.fictiveAge),
  );
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const { phase, load: loadPhase } = usePhaseOne(agent.id);
  const draftRef = useRef(draft);
  const persistedRef = useRef(persisted);
  const fictiveAgeTextRef = useRef(fictiveAgeText);
  draftRef.current = draft;
  persistedRef.current = persisted;
  fictiveAgeTextRef.current = fictiveAgeText;
  useEffect(() => {
    const switchedAgent = persistedRef.current.id !== agent.id;
    const draftIsDirty = profileDraftIsDirty(
      draftRef.current,
      persistedRef.current,
      fictiveAgeTextRef.current,
    );
    if (switchedAgent || !draftIsDirty) {
      setPersisted(agent);
      persistedRef.current = agent;
      setDraft(agent);
      draftRef.current = agent;
      setFictiveAgeText(String(agent.fictiveAge));
      setError(null);
      setSaved(false);
    }
  }, [agent]);
  const isDirty = profileDraftIsDirty(draft, persisted, fictiveAgeText);
  async function save() {
    const fictiveAge = Number(fictiveAgeText);
    if (
      !fictiveAgeText.trim() ||
      !Number.isInteger(fictiveAge) ||
      fictiveAge < 0 ||
      fictiveAge > 10000
    ) {
      setError("Informe uma idade fictícia válida.");
      return;
    }
    const prepared = withInitialTraitDefaults({
      ...draft,
      fictiveAge,
      gender: isHumanCompatibleSpecies(draft.species)
        ? draft.gender?.trim() || null
        : null,
      sexuality: isHumanCompatibleSpecies(draft.species)
        ? draft.sexuality?.trim() || null
        : null,
    });
    const validation = profileValidationError(prepared);
    if (validation !== null) {
      setError(validation);
      return;
    }
    if (
      !draft.name.trim() ||
      !draft.birthday ||
      !draft.species.trim() ||
      !draft.pronouns.trim()
    ) {
      setError("Preencha nome, data, espécie e pronomes.");
      return;
    }
    try {
      await invoke("update_agent_profile", { agent: prepared });
      setDraft(prepared);
      setPersisted(prepared);
      setFictiveAgeText(String(prepared.fictiveAge));
      setSaved(true);
      done();
    } catch {
      setError("Não foi possível salvar o perfil.");
    }
  }
  return (
    <section className="profile-form" aria-label="Perfil do agente">
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">Identidade do agente</p>
          <h1>{`Perfil de ${agent.name}`}</h1>
          <span>Detalhes, descrição e traços que orientam este agente.</span>
        </div>
      </header>
      <section className="profile-section">
        <h2>Identidade e detalhes</h2>
        <div className="profile-fields">
          <ProfileFields
            draft={draft}
            onChange={(update) => setDraft(update)}
            fictiveAgeText={fictiveAgeText}
            onFictiveAgeTextChange={setFictiveAgeText}
          />
        </div>
      </section>
      <section className="profile-section profile-default-model">
        <h2>Modelo padrão</h2>
        <p>Novas conversas deste agente começam com este modelo.</p>
        <ModelPicker
          label={`Modelo padrão de ${agent.name}`}
          ariaLabel={`Modelo padrão de ${agent.name}`}
          models={phase?.provider.models ?? []}
          value={phase?.defaultModelRef ?? null}
          providerState={phase?.provider.state ?? "checking"}
          disabled={phase === null || phase.provider.models.length === 0}
          onSelect={async (modelRef) => {
            if (modelRef === null) return;
            await invoke("select_phase_one_model", {
              agentId: agent.id,
              modelRef,
            });
            await loadPhase();
          }}
        />
      </section>
      {error ? <p role="alert">{error}</p> : null}
      {saved ? <p role="status">Perfil salvo.</p> : null}
      <div className="profile-actions">
        <button type="button" onClick={() => void save()}>
          Salvar alterações
        </button>
        <button
          type="button"
          className="secondary-action"
          disabled={!isDirty}
          onClick={() => {
            setDraft(persisted);
            setFictiveAgeText(String(persisted.fictiveAge));
            setError(null);
            setSaved(false);
          }}
        >
          Cancelar
        </button>
      </div>
    </section>
  );
}

function OnboardingForm({
  agents,
  done,
}: {
  agents: ProvisionalAgent[];
  done: () => void;
}) {
  const [drafts, setDrafts] = useState(agents);
  const [persisted, setPersisted] = useState(agents);
  const [fictiveAgeTexts, setFictiveAgeTexts] = useState(() =>
    agents.map((agent) => String(agent.fictiveAge)),
  );
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const currentAgents = useRef(agents);
  currentAgents.current = agents;
  const agentIds = agents.map((agent) => agent.id).join("\0");
  useEffect(() => {
    const next = currentAgents.current;
    setDrafts(next);
    setPersisted(next);
    setFictiveAgeTexts(next.map((agent) => String(agent.fictiveAge)));
    setError(null);
    setSaved(false);
  }, [agentIds]);
  const update = (index: number, updateAgent: ProfileDraftUpdater) =>
    setDrafts((current) =>
      current.map((agent, currentIndex) =>
        currentIndex === index ? updateAgent(agent) : agent,
      ),
    );
  async function save() {
    const prepared = drafts.map((agent, index) =>
      withInitialTraitDefaults({
        ...agent,
        fictiveAge: Number(fictiveAgeTexts[index] ?? ""),
        gender: isHumanCompatibleSpecies(agent.species)
          ? agent.gender?.trim() || null
          : null,
        sexuality: isHumanCompatibleSpecies(agent.species)
          ? agent.sexuality?.trim() || null
          : null,
      }),
    );
    if (prepared.some((agent) => profileValidationError(agent) !== null)) {
      setError("Revise a data, a idade e os traços dos dois agentes.");
      return;
    }
    if (
      drafts.length !== 2 ||
      drafts.some(
        (agent) =>
          !agent.name.trim() ||
          !agent.birthday ||
          !agent.ageCategory.trim() ||
          !agent.species.trim() ||
          !agent.pronouns.trim(),
      )
    ) {
      setError("Preencha os campos obrigatórios dos dois agentes.");
      return;
    }
    try {
      await invoke("complete_phase_two_onboarding", { agents: prepared });
      setDrafts(prepared);
      setPersisted(prepared);
      setSaved(true);
      done();
    } catch {
      setError("Não foi possível concluir a criação dos perfis.");
    }
  }
  return (
    <section
      className="profile-form onboarding-form"
      aria-label="Criação dos perfis"
    >
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">Primeiro passo</p>
          <h1>Crie os dois perfis</h1>
          <span>
            Você poderá ajustar cada agente depois, em seu próprio perfil.
          </span>
        </div>
      </header>
      {drafts.map((agent, index) => (
        <fieldset className="profile-section" key={agent.id}>
          <legend>{agent.name}</legend>
          <div className="profile-fields">
            <ProfileFields
              draft={agent}
              onChange={(updateAgent) => update(index, updateAgent)}
              fictiveAgeText={fictiveAgeTexts[index]}
              onFictiveAgeTextChange={(value) =>
                setFictiveAgeTexts((current) =>
                  current.map((text, textIndex) =>
                    textIndex === index ? value : text,
                  ),
                )
              }
            />
          </div>
        </fieldset>
      ))}
      {error ? <p role="alert">{error}</p> : null}
      {saved ? <p role="status">Perfis salvos.</p> : null}
      <div className="profile-actions">
        <button type="button" onClick={() => void save()}>
          Salvar perfis
        </button>
        <button
          type="button"
          className="secondary-action"
          disabled={JSON.stringify(drafts) === JSON.stringify(persisted)}
          onClick={() => {
            setDrafts(persisted);
            setFictiveAgeTexts(
              persisted.map((agent) => String(agent.fictiveAge)),
            );
            setError(null);
            setSaved(false);
          }}
        >
          Cancelar
        </button>
      </div>
    </section>
  );
}

export type ConversationMenuPlacement = "above" | "below";

export function conversationMenuPosition(
  triggerRect: Pick<DOMRect, "top" | "bottom" | "right">,
  menuWidth: number,
  menuHeight: number,
  viewportWidth = window.innerWidth,
  viewportHeight = window.innerHeight,
): { top: number; left: number; placement: ConversationMenuPlacement } {
  const margin = 8;
  const gap = 4;
  const fitsBelow =
    triggerRect.bottom + gap + menuHeight <= viewportHeight - margin;
  return {
    top: fitsBelow
      ? triggerRect.bottom + gap
      : Math.max(margin, triggerRect.top - gap - menuHeight),
    left: Math.min(
      Math.max(margin, triggerRect.right - menuWidth),
      Math.max(margin, viewportWidth - menuWidth - margin),
    ),
    placement: fitsBelow ? "below" : "above",
  };
}

function ConversationActionMenu({
  item,
  open,
  onOpenChange,
  onPin,
  onRename,
  onArchive,
  onRemove,
}: {
  item: PhaseOneConversation;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onPin: () => void;
  onRename: () => void;
  onArchive: () => void;
  onRemove: () => void;
}) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{
    top: number;
    left: number;
    placement: ConversationMenuPlacement;
  } | null>(null);
  const menuId = useId();

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const menu = menuRef.current;
    if (trigger === null || menu === null) return;
    const triggerRect = trigger.getBoundingClientRect();
    const menuRect = menu.getBoundingClientRect();
    setPosition(
      conversationMenuPosition(
        triggerRect,
        menuRect.width || 160,
        menuRect.height || 144,
      ),
    );
  }, []);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
    const frame = window.requestAnimationFrame(updatePosition);
    return () => window.cancelAnimationFrame(frame);
  }, [open, updatePosition]);

  useEffect(() => {
    if (!open) return;
    const focusFrame = window.requestAnimationFrame(() =>
      menuRef.current
        ?.querySelector<HTMLButtonElement>('[role="menuitem"]')
        ?.focus(),
    );
    function closeFromOutside(event: PointerEvent) {
      const target = event.target;
      if (
        target instanceof Node &&
        !triggerRef.current?.contains(target) &&
        !menuRef.current?.contains(target)
      ) {
        onOpenChange(false);
      }
    }
    function reposition() {
      updatePosition();
    }
    document.addEventListener("pointerdown", closeFromOutside);
    document.addEventListener("scroll", reposition, true);
    window.addEventListener("resize", reposition);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      document.removeEventListener("pointerdown", closeFromOutside);
      document.removeEventListener("scroll", reposition, true);
      window.removeEventListener("resize", reposition);
    };
  }, [onOpenChange, open, updatePosition]);

  function closeMenu(restoreFocus = false) {
    onOpenChange(false);
    setPosition(null);
    if (restoreFocus) triggerRef.current?.focus();
  }

  function handleMenuKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    const buttons = Array.from(
      menuRef.current?.querySelectorAll<HTMLButtonElement>(
        '[role="menuitem"]',
      ) ?? [],
    );
    const currentIndex = buttons.indexOf(
      document.activeElement as HTMLButtonElement,
    );
    if (event.key === "Escape") {
      event.preventDefault();
      closeMenu(true);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (buttons.length === 0) return;
      const delta = event.key === "ArrowDown" ? 1 : -1;
      buttons[
        (currentIndex + delta + buttons.length) % buttons.length
      ]?.focus();
    } else if (event.key === "Home") {
      event.preventDefault();
      buttons[0]?.focus();
    } else if (event.key === "End") {
      event.preventDefault();
      buttons.at(-1)?.focus();
    }
  }

  return (
    <div className="conversation-actions">
      <button
        ref={triggerRef}
        type="button"
        className="conversation-actions-trigger"
        aria-label={`Ações de ${item.title}`}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => {
          setPosition(null);
          onOpenChange(!open);
        }}
      >
        …
      </button>
      {open
        ? createPortal(
            <div
              ref={menuRef}
              id={menuId}
              className="conversation-actions-menu"
              data-placement={position?.placement ?? "below"}
              role="menu"
              aria-label={`Ações de ${item.title}`}
              style={
                position === null
                  ? { top: 0, left: 0, visibility: "hidden" }
                  : { top: position.top, left: position.left }
              }
              onKeyDown={handleMenuKeyDown}
            >
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  closeMenu(true);
                  onPin();
                }}
              >
                {item.isPinned ? "Desafixar" : "Fixar"}
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  closeMenu(true);
                  onRename();
                }}
              >
                Renomear
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  closeMenu(true);
                  onArchive();
                }}
              >
                Arquivar
              </button>
              <button
                type="button"
                role="menuitem"
                className="danger-action"
                onClick={() => {
                  closeMenu(true);
                  onRemove();
                }}
              >
                Excluir
              </button>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

export function ConversationList({
  agentId,
  changed,
  onNewDraft,
  onSelectExisting,
}: {
  agentId: string;
  changed: () => void;
  onNewDraft?: () => void;
  onSelectExisting?: () => void;
}) {
  const [items, setItems] = useState<PhaseOneConversation[]>([]);
  const [archived, setArchived] = useState<PhaseOneConversation[]>([]);
  const [manageArchived, setManageArchived] = useState(false);
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [pendingRemoval, setPendingRemoval] =
    useState<PhaseOneConversation | null>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);
  const load = useCallback(async () => {
    const next = await invoke<PhaseOneConversation[]>(
      "list_agent_conversations",
      {
        agentId,
      },
    );
    setItems(next);
  }, [agentId]);
  useEffect(() => {
    load();
  }, [load]);
  useEffect(() => {
    if (renamingId !== null) renameInputRef.current?.focus();
  }, [renamingId]);
  async function select(conversationId: string) {
    onSelectExisting?.();
    await invoke("set_active_agent_conversation", { agentId, conversationId });
    changed();
  }
  async function loadArchived() {
    const previous = await invoke<PhaseOneConversation[]>(
      "list_archived_agent_conversations",
      { agentId },
    );
    setArchived(previous);
  }
  async function rename(item: PhaseOneConversation) {
    if (!renameValue.trim()) return;
    await invoke("rename_agent_conversation", {
      agentId,
      conversationId: item.id,
      title: renameValue,
    });
    setRenamingId(null);
    setOpenMenuId(null);
    await load();
    changed();
  }
  async function archive(item: PhaseOneConversation) {
    await invoke("archive_agent_conversation", {
      agentId,
      conversationId: item.id,
    });
    await load();
    changed();
  }
  function remove(item: PhaseOneConversation) {
    setPendingRemoval(item);
  }
  async function confirmRemoval() {
    const item = pendingRemoval;
    if (item === null) return;
    setPendingRemoval(null);
    await invoke("delete_agent_conversation", {
      agentId,
      conversationId: item.id,
    });
    await load();
    changed();
  }
  async function pin(item: PhaseOneConversation) {
    await invoke("pin_agent_conversation", {
      agentId,
      conversationId: item.id,
      pinned: !item.isPinned,
    });
    await load();
    changed();
  }
  async function restore(item: PhaseOneConversation) {
    await invoke("restore_agent_conversation", {
      agentId,
      conversationId: item.id,
    });
    await loadArchived();
    await load();
    changed();
  }
  return (
    <div
      className="conversation-list"
      role="region"
      aria-label="Conversas do agente"
    >
      <div className="conversation-list-heading">
        <span>Conversas recentes</span>
        <small>Fixadas primeiro</small>
      </div>
      {items.map((item) => (
        <div
          key={item.id}
          className="conversation-list-item"
          data-pinned={item.isPinned}
        >
          <button
            type="button"
            className="conversation-list-select"
            onClick={() => void select(item.id)}
          >
            {item.isPinned ? "★ " : ""}
            {item.title}
          </button>
          <ConversationActionMenu
            item={item}
            open={openMenuId === item.id}
            onOpenChange={(open) => setOpenMenuId(open ? item.id : null)}
            onPin={() => void pin(item)}
            onRename={() => {
              setRenamingId(item.id);
              setRenameValue(item.title);
            }}
            onArchive={() => void archive(item)}
            onRemove={() => remove(item)}
          />
          {renamingId === item.id ? (
            <form
              className="conversation-rename"
              onSubmit={(event) => {
                event.preventDefault();
                void rename(item);
              }}
            >
              <input
                ref={renameInputRef}
                aria-label={`Novo nome de ${item.title}`}
                value={renameValue}
                maxLength={160}
                onChange={(event) => setRenameValue(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Escape") setRenamingId(null);
                }}
              />
              <div className="conversation-rename-actions">
                <button type="submit">Salvar nome</button>
                <button type="button" onClick={() => setRenamingId(null)}>
                  Cancelar
                </button>
              </div>
            </form>
          ) : null}
        </div>
      ))}
      <button
        type="button"
        className="conversation-archive-management"
        onClick={() => {
          setManageArchived((value) => !value);
          if (!manageArchived) void loadArchived();
        }}
      >
        {manageArchived ? "Fechar arquivadas" : "Gerenciar arquivadas"}
      </button>
      {manageArchived ? (
        <div className="conversation-list-archived">
          <strong>Conversas arquivadas</strong>
          {archived.length === 0 ? (
            <span>Nenhuma conversa arquivada.</span>
          ) : null}
          {archived.map((item) => (
            <div key={item.id} className="conversation-list-item">
              <span className="conversation-list-title">{item.title}</span>
              <button
                type="button"
                className="conversation-list-action"
                onClick={() => void restore(item)}
              >
                Restaurar
              </button>
            </div>
          ))}
        </div>
      ) : null}
      <button
        type="button"
        className="conversation-list-create"
        onClick={() => onNewDraft?.()}
      >
        Nova conversa
      </button>
      {pendingRemoval ? (
        <ConfirmDialog
          title="Excluir conversa?"
          description={`As mensagens de “${pendingRemoval.title}” também serão removidas.`}
          confirmLabel="Excluir conversa"
          onCancel={() => setPendingRemoval(null)}
          onConfirm={() => void confirmRemoval()}
        />
      ) : null}
    </div>
  );
}

const memoryCategoryHelp: Record<string, string> = {
  fact: "Um dado estável sobre a pessoa ou o agente.",
  preference: "Uma preferência que pode orientar respostas futuras.",
  rule: "Uma regra explícita para preservar ao conversar.",
  emotional:
    "Uma lembrança afetiva, sem transformar sentimento em diagnóstico.",
  permanent: "Um registro durável que o Owner escolheu manter.",
};

export function MemoryWorkspace({ agentId }: { agentId: string }) {
  const [items, setItems] = useState<AgentMemory[]>([]);
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("preference");
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("active");
  const load = useCallback(
    () =>
      void invoke<AgentMemory[]>("search_agent_memories", {
        agentId,
        query: query || null,
        status: status === "all" ? null : status,
        category: null,
        sourceType: null,
      }).then(setItems),
    [agentId, query, status],
  );
  useEffect(() => {
    load();
  }, [load]);

  async function save(confirmed = true) {
    if (!content.trim()) return;
    await invoke("create_agent_memory", {
      agentId,
      category,
      content,
      confirmed,
    });
    setContent("");
    load();
  }

  async function updateStatus(memoryId: string, nextStatus: string) {
    await invoke("set_agent_memory_status", {
      agentId,
      memoryId,
      status: nextStatus,
    });
    load();
  }

  return (
    <section className="memory-workspace" aria-label="Memórias do agente">
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">Conhecimento do agente</p>
          <h2>Memórias</h2>
          <span>Fatos, preferências e regras ficam separados por agente.</span>
        </div>
        <span className="workspace-count">
          {items.length} {status === "active" ? "memórias ativas" : "memórias"}
        </span>
      </header>
      <p className="memory-guidance">
        As memórias pertencem somente a este agente e podem influenciar o
        contexto das conversas. O Owner decide o que fica salvo.
      </p>
      <div className="memory-filters">
        <label>
          Buscar
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Buscar memórias"
          />
        </label>
        <label>
          Mostrar
          <select
            value={status}
            onChange={(event) => setStatus(event.target.value)}
          >
            <option value="active">Ativas</option>
            <option value="archived">Arquivadas</option>
            <option value="trashed">Lixeira</option>
            <option value="candidate_rejected">Rejeitadas</option>
            <option value="all">Todas</option>
          </select>
        </label>
      </div>
      <div className="memory-records">
        {items.map((item) => (
          <article className="memory-card" key={item.id}>
            <div className="memory-card-heading">
              <span>{item.category}</span>
              <small>
                {item.confirmationStatus === "pending"
                  ? "Pendente"
                  : "Confirmada"}
              </small>
            </div>
            <p>{item.content}</p>
            <div className="memory-card-actions">
              {item.confirmationStatus === "pending" ? (
                <>
                  <button
                    type="button"
                    onClick={() => void updateStatus(item.id, "active")}
                  >
                    Confirmar
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      void updateStatus(item.id, "candidate_rejected")
                    }
                  >
                    Rejeitar
                  </button>
                </>
              ) : null}
              {item.status === "active" ? (
                <button
                  type="button"
                  onClick={() => void updateStatus(item.id, "archived")}
                >
                  Arquivar
                </button>
              ) : item.status === "archived" || item.status === "trashed" ? (
                <button
                  type="button"
                  onClick={() => void updateStatus(item.id, "active")}
                >
                  Restaurar
                </button>
              ) : null}
            </div>
          </article>
        ))}
        {items.length === 0 ? (
          <p className="workspace-empty">Nenhuma memória encontrada aqui.</p>
        ) : null}
      </div>
      <form
        className="memory-composer"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <div>
          <p className="eyebrow">Adicionar</p>
          <h3>Nova memória</h3>
        </div>
        <label>
          Categoria
          <select
            value={category}
            onChange={(event) => setCategory(event.target.value)}
          >
            <option value="fact">Fato</option>
            <option value="preference">Preferência</option>
            <option value="rule">Regra</option>
            <option value="emotional">Lembrança afetiva</option>
            <option value="permanent">Permanente</option>
          </select>
          <small className="memory-category-help">
            {memoryCategoryHelp[category]}
          </small>
        </label>
        <label className="memory-content-field">
          Conteúdo
          <textarea
            value={content}
            onChange={(event) => setContent(event.target.value)}
            placeholder="O que este agente deve lembrar?"
            maxLength={4000}
          />
        </label>
        <div className="memory-composer-actions">
          <button type="submit" title="Salvar como memória durável do Owner">
            Salvar memória
          </button>
          <button
            type="button"
            title="Criar uma proposta que precisa de confirmação"
            onClick={() => void save(false)}
          >
            Propor memória
          </button>
        </div>
      </form>
      <p className="memory-guidance">
        “Salvar memória” grava uma memória durável. “Propor memória” cria uma
        candidata pendente para o Owner confirmar ou rejeitar.
      </p>
    </section>
  );
}

export function AgentStateControls({ agentId }: { agentId: string }) {
  const [state, setState] = useState<AgentSimulatedState | null>(null);
  const [saving, setSaving] = useState(false);
  const load = useCallback(
    () =>
      void invoke<AgentSimulatedState>("get_agent_simulated_state", { agentId })
        .then(setState)
        .catch(() => setState(null)),
    [agentId],
  );

  useEffect(() => {
    load();
  }, [load]);

  async function update(action: () => Promise<unknown>) {
    if (saving) return;
    setSaving(true);
    try {
      await action();
      load();
    } finally {
      setSaving(false);
    }
  }

  if (state === null) return null;
  return (
    <section className="agent-state-controls" aria-label="Estado do agente">
      <strong>Estado</strong>
      <p className="state-guidance">
        Normal permite texto e voz configurada conforme as guardas do Rust e do
        provedor; Sem voz mantém o texto e silencia a voz sintetizada;
        Silencioso bloqueia conversas cognitivas/públicas iniciadas pelo agente
        e alterações de configurações de voz. Texto direto continua sujeito às
        guardas normais, inclusive suspensão. Energia, humor e sono são valores
        fictícios simulados — não são medições de saúde.
      </p>
      <label>
        Modo
        <select
          value={state.mode}
          disabled={saving}
          onChange={(event) =>
            void update(() =>
              invoke("set_agent_simulated_mode", {
                agentId,
                mode: event.target.value,
              }),
            )
          }
        >
          <option value="normal">Normal</option>
          <option value="voice_muted">Sem voz</option>
          <option value="silent">Silencioso</option>
        </select>
      </label>
      <small>
        Energia {state.energy}% · Humor {state.mood}% · Sono {state.sleep}%
      </small>
      <button
        type="button"
        disabled={saving}
        onClick={() =>
          void update(() =>
            invoke("set_agent_suspension", {
              agentId,
              suspended: !state.suspended,
            }),
          )
        }
      >
        {state.suspended ? "Retomar agente" : "Suspender agente"}
      </button>
      <button
        type="button"
        disabled={saving || !state.suspended}
        onClick={() => void update(() => invoke("wake_agent_now", { agentId }))}
      >
        Acordar agora
      </button>
      <small>
        Suspender pausa o avanço simulado; Retomar permite que ele continue.
        “Acordar agora” ajusta somente o sono e a energia fictícios por um
        período temporário, sem remover a suspensão ou acordar uma pessoa real.
      </small>
    </section>
  );
}

const voiceErrorCopy: Record<string, string> = {
  operation_unavailable:
    "Voz local indisponível; a conversa de texto continua disponível.",
  conversation_temporary_blocked:
    "A conversa temporária não pode salvar configurações ou consentimento de voz.",
  voice_blocked_silent: "O modo silencioso bloqueia alterações de voz.",
  voice_blocked_suspended:
    "Agentes suspensos não alteram configurações de voz.",
  voice_reference_invalid:
    "Use uma referência local fixture: ou local: válida.",
  voice_consent_invalid:
    "O consentimento exige uma referência fixture:custom- ou local:custom- válida.",
  invalid_idempotency_key: "Não foi possível repetir a operação com segurança.",
};

function voiceAvailabilityCopy(settings: VoiceSettings): string {
  if (settings.voiceMuted) {
    return "Modo sem voz: nenhuma saída audível será produzida; a conversa de texto continua disponível.";
  }
  if (
    settings.inputDeviceRef === null ||
    settings.outputDeviceRef === null ||
    settings.recognitionModelRef === null ||
    settings.synthesisModelRef === null
  ) {
    return "Voz degradada: selecione dispositivos e modelos locais; a conversa de texto continua disponível.";
  }
  if (settings.silent) {
    return "Modo silencioso: não há ação espontânea; solicitações diretas continuam sem quebrar a conversa de texto.";
  }
  return "Voz local configurada. Nenhum áudio bruto é salvo neste checkpoint.";
}

export function VoiceControls({
  agentId,
  temporaryChat,
  safeMode = false,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode?: boolean;
}) {
  const [settings, setSettings] = useState<VoiceSettings | null>(null);
  const [recognitionModelRef, setRecognitionModelRef] = useState("");
  const [synthesisModelRef, setSynthesisModelRef] = useState("");
  const [inputDeviceRef, setInputDeviceRef] = useState("");
  const [outputDeviceRef, setOutputDeviceRef] = useState("");
  const [voiceDevices, setVoiceDevices] = useState<VoiceDevice[]>([]);
  const [localProviders, setLocalProviders] = useState<LocalProvider[]>([]);
  const [providerKind, setProviderKind] = useState<LocalProviderKind>("stt");
  const [providerId, setProviderId] = useState("");
  const [providerName, setProviderName] = useState("");
  const [providerPath, setProviderPath] = useState("");
  const [providerProtocol, setProviderProtocol] = useState("aip-voice-v1");
  const [customVoiceRef, setCustomVoiceRef] = useState(
    "fixture:custom-neutral-v1",
  );
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [activeOperation, setActiveOperation] = useState<string | null>(null);
  const blocked = temporaryChat || safeMode;
  const load = useCallback(async () => {
    try {
      const next = await invoke<VoiceSettings>("get_voice_settings", {
        agentId,
      });
      setSettings(next);
      setRecognitionModelRef(next.recognitionModelRef ?? "");
      setSynthesisModelRef(next.synthesisModelRef ?? "");
      setInputDeviceRef(next.inputDeviceRef ?? "");
      setOutputDeviceRef(next.outputDeviceRef ?? "");
      setCustomVoiceRef(next.customVoiceRef ?? "fixture:custom-neutral-v1");
      setError(null);
    } catch (cause) {
      setSettings(null);
      setError(
        voiceErrorCopy[String(cause)] ||
          "Voz local indisponível; a conversa de texto continua disponível.",
      );
    }
  }, [agentId]);

  useEffect(() => {
    void load();
    void invoke<unknown[]>("list_voice_devices")
      .then((values) => {
        const devices = values
          .map(parseVoiceDevice)
          .filter((device): device is VoiceDevice => device !== null);
        setVoiceDevices(devices.slice(0, 64));
      })
      .catch(() => setVoiceDevices([]));
    void invoke<unknown>("list_local_providers")
      .then((value) => setLocalProviders(parseLocalProviders(value) ?? []))
      .catch(() => setLocalProviders([]));
  }, [load]);

  async function registerProvider() {
    if (blocked || busy) return;
    if (!providerId.trim() || !providerName.trim() || !providerPath.trim()) {
      setError("Informe identificador, nome e caminho absoluto do provedor.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const request: LocalProviderRequest = {
        agentId,
        id: providerId.trim(),
        kind: providerKind,
        displayName: providerName.trim(),
        executablePath: providerPath.trim(),
        protocolVersion:
          providerKind === "visual" ? "aip-screen-vision-v1" : providerProtocol,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      };
      const provider = await invoke<LocalProvider>(
        "register_local_provider",
        request,
      );
      setLocalProviders((current) =>
        [...current.filter((entry) => entry.id !== provider.id), provider].sort(
          (a, b) => a.displayName.localeCompare(b.displayName),
        ),
      );
      setStatus(
        `Provedor local ${provider.displayName} validado e disponível.`,
      );
      setProviderId("");
      setProviderName("");
      setProviderPath("");
    } catch (cause) {
      setError(String(cause) || "O provedor local foi recusado.");
    } finally {
      setBusy(false);
    }
  }

  async function disableProvider(provider: LocalProvider) {
    if (blocked || busy) return;
    setBusy(true);
    setError(null);
    try {
      const request: LocalProviderIdRequest = {
        agentId,
        id: provider.id,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      };
      const disabled = await invoke<LocalProvider>(
        "disable_local_provider",
        request,
      );
      setLocalProviders((current) =>
        current.map((entry) => (entry.id === disabled.id ? disabled : entry)),
      );
      setStatus(`Provedor ${provider.displayName} desativado.`);
    } catch (cause) {
      setError(String(cause) || "Não foi possível desativar o provedor.");
    } finally {
      setBusy(false);
    }
  }

  async function saveSettings() {
    if (blocked || settings === null) return;
    setBusy(true);
    setError(null);
    try {
      const request: VoiceSettingsRequest = {
        agentId,
        recognitionModelRef: recognitionModelRef || null,
        synthesisModelRef: synthesisModelRef || null,
        inputDeviceRef: inputDeviceRef || null,
        outputDeviceRef: outputDeviceRef || null,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      };
      await invoke<VoiceSettings>("update_voice_settings", request);
      setStatus("Referências locais de voz salvas.");
      await load();
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] || "A configuração de voz foi recusada.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function changeConsent(granted: boolean) {
    if (blocked || settings === null) return;
    setBusy(true);
    setError(null);
    try {
      const request: CustomVoiceConsentRequest = {
        agentId,
        granted,
        customVoiceRef: granted ? customVoiceRef || null : null,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      };
      await invoke<VoiceSettings>("set_custom_voice_consent", request);
      setStatus(
        granted
          ? "Consentimento customizado registrado."
          : "Consentimento customizado revogado.",
      );
      await load();
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] || "O consentimento de voz foi recusado.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function testTranscription() {
    setBusy(true);
    setError(null);
    try {
      const request: VoiceTranscriptionRequest = {
        agentId,
        fixtureId: "fixture:hello",
        temporaryChat,
      };
      const result = await invoke<VoiceTranscriptionResult>(
        "transcribe_voice_fixture",
        request,
      );
      setStatus(
        result.status === "ready"
          ? `Fixture transcrita: ${result.text}`
          : `Voz degradada (${result.code}); a conversa de texto continua disponível.`,
      );
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] ||
          "A transcrição local está indisponível.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function testSynthesis() {
    setBusy(true);
    setError(null);
    try {
      const request: VoiceSynthesisRequest = {
        agentId,
        text: "Olá, fixture local.",
        temporaryChat,
      };
      const result = await invoke<VoiceSynthesisResult>(
        "synthesize_voice_fixture",
        request,
      );
      setStatus(
        result.status === "muted"
          ? "Saída de voz silenciada; a conversa de texto continua disponível."
          : result.status === "ready"
            ? "Fixture sintetizada somente como metadados; nenhum áudio foi salvo."
            : `Voz degradada (${result.code}); a conversa de texto continua disponível.`,
      );
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] || "A síntese local está indisponível.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function runLocalCapture(kind: "transcription" | "wake_word") {
    if (blocked || busy) return;
    const operationId = crypto.randomUUID();
    const request: VoiceCaptureRuntimeRequest = {
      agentId,
      operationId,
      idempotencyKey: crypto.randomUUID(),
      durationMs: 1000,
      temporaryChat: false,
    };
    setBusy(true);
    setActiveOperation(operationId);
    setStatus(
      "Operação local iniciada; aguardando dispositivo/provedor configurado.",
    );
    setError(null);
    try {
      const result = await invoke<
        VoiceRuntimeTranscriptionResult | VoiceRuntimeWakeWordResult
      >(
        kind === "transcription"
          ? "transcribe_voice_local"
          : "detect_voice_wake_word_local",
        request,
      );
      setStatus(
        `${kind === "transcription" ? "Transcrição" : "Wake-word"} ${result.status}${
          result.code ? ` (${result.code})` : ""
        }; conversa de texto continua disponível.`,
      );
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] ||
          "A operação local está indisponível; a conversa de texto continua disponível.",
      );
    } finally {
      setActiveOperation(null);
      setBusy(false);
    }
  }

  async function runLocalSynthesis() {
    if (blocked || busy) return;
    const operationId = crypto.randomUUID();
    const request = {
      agentId,
      operationId,
      idempotencyKey: crypto.randomUUID(),
      text: "Olá, operação local.",
      temporaryChat: false,
    };
    setBusy(true);
    setActiveOperation(operationId);
    setStatus(
      "Síntese local iniciada; aguardando dispositivo/provedor configurado.",
    );
    setError(null);
    try {
      const result = await invoke<VoiceRuntimeSynthesisResult>(
        "synthesize_voice_local",
        request,
      );
      setStatus(
        `Síntese ${result.status}${result.code ? ` (${result.code})` : ""}; conversa de texto continua disponível.`,
      );
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] ||
          "A operação local está indisponível; a conversa de texto continua disponível.",
      );
    } finally {
      setActiveOperation(null);
      setBusy(false);
    }
  }

  async function cancelLocalOperation() {
    if (!activeOperation) return;
    const request: VoiceOperationCancellationRequest = {
      agentId,
      operationId: activeOperation,
    };
    try {
      await invoke<boolean>("cancel_voice_operation", request);
      setStatus("Cancelamento solicitado para a operação local.");
    } catch (cause) {
      setError(
        voiceErrorCopy[String(cause)] ||
          "Não foi possível cancelar a operação local.",
      );
    }
  }

  if (settings === null) {
    return (
      <section aria-label="Configurações de voz">
        <strong>Voz local</strong>
        <p role="alert">
          {error ||
            "Voz local indisponível; a conversa de texto continua disponível."}
        </p>
      </section>
    );
  }

  return (
    <section className="voice-controls" aria-label="Configurações de voz">
      <div className="voice-status-block">
        <h3>Status</h3>
        <strong>Voz local</strong>
        <p>{voiceAvailabilityCopy(settings)}</p>
      </div>
      {temporaryChat ? (
        <p role="status">
          Conversa temporária: configurações e consentimento de voz não serão
          salvos.
        </p>
      ) : null}
      {safeMode ? (
        <p role="status">
          Modo seguro: operações de voz bloqueadas; a conversa de texto continua
          disponível.
        </p>
      ) : null}
      <section className="voice-section">
        <h3>Provedores</h3>
        <p>
          STT e TTS usam provedores locais substituíveis.{" "}
          {localProviders.length === 0
            ? "Nenhum provedor configurado."
            : `${localProviders.length} provedor(es) local(is) disponível(is).`}
        </p>
      </section>
      <details className="voice-advanced">
        <summary>Avançado: configurar provedores</summary>
        <p>
          Cadastre um executável local com protocolo explícito. O caminho é
          validado no dispositivo; nenhuma variável de ambiente é necessária
          para a configuração normal.
        </p>
        <div className="inline-form">
          <label>
            Tipo
            <select
              value={providerKind}
              disabled={blocked || busy}
              onChange={(event) => {
                const kind = event.target.value as LocalProviderKind;
                setProviderKind(kind);
                setProviderProtocol(
                  kind === "visual" ? "aip-screen-vision-v1" : "aip-voice-v1",
                );
              }}
            >
              <option value="stt">Voz — transcrição</option>
              <option value="tts">Voz — síntese</option>
              <option value="visual">Visão de tela</option>
            </select>
          </label>
          <label>
            Identificador
            <input
              value={providerId}
              maxLength={96}
              disabled={blocked || busy}
              onChange={(event) => setProviderId(event.target.value)}
              placeholder="meu-provedor"
            />
          </label>
          <label>
            Nome exibido
            <input
              value={providerName}
              maxLength={120}
              disabled={blocked || busy}
              onChange={(event) => setProviderName(event.target.value)}
              placeholder="Meu provedor local"
            />
          </label>
          <label>
            Caminho absoluto do executável
            <input
              value={providerPath}
              maxLength={1024}
              disabled={blocked || busy}
              onChange={(event) => setProviderPath(event.target.value)}
              placeholder="C:\\Ferramentas\\provedor.exe"
            />
          </label>
          <button
            type="button"
            disabled={blocked || busy}
            onClick={() => void registerProvider()}
          >
            Validar e registrar
          </button>
        </div>
        {localProviders.length === 0 ? (
          <p>
            Nenhum provedor registrado. Voz e visão permanecem degradadas até a
            configuração local.
          </p>
        ) : (
          <ul aria-label="Provedores locais registrados">
            {localProviders.map((provider) => (
              <li key={provider.id}>
                <strong>{provider.displayName}</strong> ({provider.kind}) —{" "}
                {provider.validationStatus}: {provider.validationResult}{" "}
                {provider.enabled ? (
                  <>
                    <button
                      type="button"
                      disabled={blocked || busy}
                      onClick={() => {
                        if (provider.kind === "stt")
                          setRecognitionModelRef(`local:stt:${provider.id}`);
                        if (provider.kind === "tts")
                          setSynthesisModelRef(`local:tts:${provider.id}`);
                        setStatus(
                          `Referência ${provider.displayName} selecionada; salve as configurações.`,
                        );
                      }}
                    >
                      Usar
                    </button>{" "}
                    <button
                      type="button"
                      disabled={blocked || busy}
                      onClick={() => void disableProvider(provider)}
                    >
                      Desativar
                    </button>
                  </>
                ) : (
                  <span>desativado</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </details>
      <section className="voice-section">
        <h3>Dispositivos</h3>
        <div className="voice-device-grid">
          <label>
            Microfone
            <select
              value={inputDeviceRef}
              disabled={blocked || busy}
              onChange={(event) => setInputDeviceRef(event.target.value)}
            >
              <option value="">Selecionar microfone</option>
              {voiceDevices
                .filter((device) => device.direction === "input")
                .map((device) => (
                  <option key={device.reference} value={device.reference}>
                    {device.displayName}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Saída de áudio
            <select
              value={outputDeviceRef}
              disabled={blocked || busy}
              onChange={(event) => setOutputDeviceRef(event.target.value)}
            >
              <option value="">Selecionar saída</option>
              {voiceDevices
                .filter((device) => device.direction === "output")
                .map((device) => (
                  <option key={device.reference} value={device.reference}>
                    {device.displayName}
                  </option>
                ))}
            </select>
          </label>
        </div>
        <button
          type="button"
          disabled={blocked || busy}
          onClick={() => {
            void invoke<unknown[]>("list_voice_devices")
              .then((values) => {
                setVoiceDevices(
                  values
                    .map(parseVoiceDevice)
                    .filter((device): device is VoiceDevice => device !== null)
                    .slice(0, 64),
                );
              })
              .catch(() => setVoiceDevices([]));
          }}
        >
          Atualizar dispositivos
        </button>
      </section>
      <details className="voice-advanced">
        <summary>Avançado: referências e consentimento</summary>
        <label>
          Modelo local de transcrição
          <input
            value={recognitionModelRef}
            placeholder="local:stt:provider"
            maxLength={160}
            disabled={blocked || busy}
            onChange={(event) => setRecognitionModelRef(event.target.value)}
          />
        </label>
        <label>
          Modelo local de síntese
          <input
            value={synthesisModelRef}
            placeholder="local:tts:provider"
            maxLength={160}
            disabled={blocked || busy}
            onChange={(event) => setSynthesisModelRef(event.target.value)}
          />
        </label>
        <label>
          Referência do microfone
          <input
            value={inputDeviceRef}
            placeholder="local:wavein:0"
            maxLength={160}
            disabled={blocked || busy}
            onChange={(event) => setInputDeviceRef(event.target.value)}
          />
        </label>
        <label>
          Referência da saída de áudio
          <input
            value={outputDeviceRef}
            placeholder="local:waveout:0"
            maxLength={160}
            disabled={blocked || busy}
            onChange={(event) => setOutputDeviceRef(event.target.value)}
          />
        </label>
        <label>
          Referência de voz customizada sintética
          <input
            value={customVoiceRef}
            placeholder="local:custom-neutral-v1"
            maxLength={160}
            disabled={blocked || busy}
            onChange={(event) => setCustomVoiceRef(event.target.value)}
          />
        </label>
        <p>Voz-base protegida: {settings.baseVoiceId}.</p>
        <button
          type="button"
          disabled={blocked || busy}
          onClick={() =>
            void changeConsent(settings.customVoiceConsent !== "granted")
          }
        >
          {settings.customVoiceConsent === "granted"
            ? "Revogar consentimento customizado"
            : "Conceder consentimento customizado"}
        </button>
      </details>
      <button
        type="button"
        disabled={blocked || busy}
        onClick={() => void saveSettings()}
      >
        Salvar referências locais
      </button>
      <button
        type="button"
        disabled={blocked || busy}
        onClick={() => void testTranscription()}
      >
        Testar transcrição de fixture
      </button>
      <button
        type="button"
        disabled={blocked || busy}
        onClick={() => void testSynthesis()}
      >
        Testar síntese de fixture
      </button>
      <p>Operações reais sob demanda (Windows, somente referências locais):</p>
      <button
        type="button"
        disabled={blocked || busy}
        onClick={() => void runLocalCapture("transcription")}
      >
        Capturar e transcrever localmente
      </button>
      <button
        type="button"
        disabled={blocked || busy}
        onClick={() => void runLocalSynthesis()}
      >
        Sintetizar localmente
      </button>
      <button
        type="button"
        disabled={blocked || busy}
        onClick={() => void runLocalCapture("wake_word")}
      >
        Verificar wake-word localmente
      </button>
      {activeOperation ? (
        <button type="button" onClick={() => void cancelLocalOperation()}>
          Cancelar operação local ({activeOperation})
        </button>
      ) : null}
      {status ? <p role="status">{status}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
    </section>
  );
}

const cognitiveErrorCopy: Record<string, string> = {
  agent_not_found: "Agente não encontrado.",
  event_not_found: "Alteração não encontrada.",
  invalid_idempotency_key: "Não foi possível repetir a operação com segurança.",
  protected_trait: "Este traço é protegido.",
  trait_not_found: "Traço não disponível.",
  invalid_value: "Valor inválido.",
  invalid_reason: "Informe um motivo válido.",
  oscillation_blocked: "A alternância recente impede esta alteração.",
  rate_limit_window: "Limite de alteração de 30 dias atingido.",
  rate_limit_event: "Alteração acima do limite permitido.",
  source_ineligible: "A evidência não é elegível.",
  source_not_found: "A evidência não foi encontrada.",
  ownership_mismatch: "Esta alteração não pertence ao agente atual.",
  idempotency_conflict: "A operação conflita com uma solicitação anterior.",
  duplicate_evidence: "Esta evidência já foi aplicada.",
  rollback_conflict: "A reversão conflita com uma alteração posterior.",
  rollback_not_allowed: "Este evento não pode ser revertido.",
  persistence_failed: "Não foi possível salvar a alteração.",
  operation_unavailable: "Operação indisponível.",
};

export function CognitivePanel({ agentId }: { agentId: string }) {
  const [traits, setTraits] = useState<CognitiveTrait[]>([]);
  const [events, setEvents] = useState<CognitiveEventSummary[]>([]);
  const [selected, setSelected] = useState<CognitiveEventExplanation | null>(
    null,
  );
  const [traitKey, setTraitKey] = useState("");
  const [value, setValue] = useState("0.5");
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const loadVersion = useRef(0);
  const activeAgentId = useRef(agentId);
  const load = useCallback(async () => {
    const version = ++loadVersion.current;
    const [nextTraits, nextEvents] = await Promise.all([
      invoke<CognitiveTrait[]>("list_cognitive_traits", { agentId }),
      invoke<CognitiveEventSummary[]>("list_cognitive_events", { agentId }),
    ]);
    if (version !== loadVersion.current) return;
    setTraits(nextTraits);
    setEvents(nextEvents);
    setTraitKey(
      (current) =>
        current || nextTraits.find((trait) => !trait.isProtected)?.key || "",
    );
  }, [agentId]);
  useEffect(() => {
    activeAgentId.current = agentId;
    setTraits([]);
    setEvents([]);
    setError(null);
    setSuccess(null);
    setLoading(true);
    void load()
      .catch(() => {
        if (activeAgentId.current === agentId) {
          setError("Não foi possível carregar os valores cognitivos.");
        }
      })
      .finally(() => {
        if (activeAgentId.current === agentId) setLoading(false);
      });
    setSelected(null);
  }, [agentId, load]);
  async function correct() {
    const numericValue = Number(value);
    if (
      !Number.isFinite(numericValue) ||
      numericValue < 0 ||
      numericValue > 1
    ) {
      setError("Informe um valor entre 0 e 1.");
      return;
    }
    if (!reason.trim()) {
      setError("Informe o motivo da correção.");
      return;
    }
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      await invoke("create_owner_trait_correction", {
        agentId,
        traitKey,
        value: numericValue,
        reason,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      });
      setReason("");
      await load();
      if (activeAgentId.current === agentId) setSuccess("Correção aplicada.");
    } catch (cause) {
      setError(cognitiveErrorCopy[String(cause)] || "A correção foi recusada.");
    } finally {
      setBusy(false);
    }
  }
  async function rollback(eventId: string) {
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      await invoke("rollback_cognitive_event", {
        agentId,
        eventId,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      });
      await load();
      if (activeAgentId.current === agentId) setSuccess("Reversão aplicada.");
    } catch (cause) {
      setError(cognitiveErrorCopy[String(cause)] || "A reversão foi recusada.");
    } finally {
      setBusy(false);
    }
  }
  async function explain(eventId: string) {
    try {
      const explanation = await invoke<CognitiveEventExplanation>(
        "explain_cognitive_event",
        { agentId, eventId },
      );
      if (activeAgentId.current === agentId) setSelected(explanation);
    } catch {
      if (activeAgentId.current === agentId) {
        setError("Não foi possível abrir a explicação.");
      }
    }
  }
  return (
    <section
      className="settings-card"
      aria-label="Valores cognitivos simulados"
    >
      <h2>Valores cognitivos simulados</h2>
      <p>
        São valores de produto simulados; não representam emoções reais nem
        diagnóstico psicológico.
      </p>
      {loading ? <p>Carregando valores cognitivos…</p> : null}
      <ul>
        {traits.map((trait) => (
          <li key={trait.key}>
            {trait.key}: {trait.value.toFixed(2)} —{" "}
            {trait.isProtected ? "protegido" : "evolutivo"}
          </li>
        ))}
      </ul>
      <label>
        Traço{" "}
        <select
          value={traitKey}
          onChange={(event) => setTraitKey(event.target.value)}
        >
          {traits
            .filter((trait) => !trait.isProtected)
            .map((trait) => (
              <option key={trait.key} value={trait.key}>
                {trait.key}
              </option>
            ))}
        </select>
      </label>
      <label>
        Valor (0 a 1)
        <input
          type="number"
          min="0"
          max="1"
          step="0.01"
          value={value}
          onChange={(event) => setValue(event.target.value)}
        />
      </label>
      <label>
        Motivo obrigatório
        <textarea
          value={reason}
          maxLength={500}
          onChange={(event) => setReason(event.target.value)}
        />
      </label>
      <button
        type="button"
        aria-label="Corrigir valor cognitivo"
        disabled={busy}
        onClick={() => void correct()}
      >
        Corrigir valor
      </button>
      {error ? <p role="alert">{error}</p> : null}
      {success ? <p role="status">{success}</p> : null}
      <h3>Histórico recente</h3>
      <ul>
        {events.map((event) => (
          <li key={event.id}>
            <button
              type="button"
              aria-label={`Explicar alteração de ${event.traitKey}`}
              onClick={() => void explain(event.id)}
            >
              {event.traitKey}: {event.resultingValue.toFixed(2)} ({event.kind})
            </button>
            {event.status === "applied" && event.kind !== "rollback" ? (
              <button
                type="button"
                aria-label={`Reverter alteração de ${event.traitKey}`}
                disabled={busy}
                onClick={() => void rollback(event.id)}
              >
                Reverter
              </button>
            ) : null}
          </li>
        ))}
      </ul>
      {selected ? (
        <p>
          {selected.traitLabel}: {selected.event.priorValue.toFixed(2)} →{" "}
          {selected.event.resultingValue.toFixed(2)}. {selected.event.reason}
        </p>
      ) : null}
      <CognitiveCorePanel agentId={agentId} />
    </section>
  );
}

export function CognitivePanelGate({
  agentId,
  temporaryChat,
  safeMode = false,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode?: boolean;
}) {
  if (temporaryChat || safeMode) {
    return (
      <p role="status" aria-label="Valores cognitivos somente para leitura">
        {temporaryChat
          ? "Conversa temporária ativa: opiniões, relacionamentos e objetivos ficam somente para leitura; nenhuma alteração será salva."
          : "Modo seguro ativo: opiniões, relacionamentos e objetivos ficam somente para leitura; nenhuma alteração será salva."}
      </p>
    );
  }
  return <CognitivePanel agentId={agentId} />;
}

const coreGoalStatusCopy: Record<CognitiveGoal["status"], string> = {
  proposed: "proposto",
  active: "ativo",
  suspended: "suspenso",
  completed: "concluído",
  cancelled: "cancelado",
  archived: "arquivado",
  rejected: "rejeitado",
};

const coreErrorCopy: Record<string, string> = {
  ownership_mismatch: "Este registro pertence a outro agente.",
  invalid_reason: "Informe um motivo válido para a operação.",
  invalid_idempotency_key: "A chave de idempotência não é válida.",
  rollback_conflict: "Somente o último evento aplicado pode ser revertido.",
  rollback_not_allowed: "Esse evento não pode ser revertido.",
  invalid_classification: "A classificação da opinião não é válida.",
  invalid_evidence: "A evidência não é válida.",
  attribution_required: "Informe a atribuição da experiência.",
  internet_fact_unverified:
    "Informações externas não podem virar fatos verificados.",
  inference_not_fact: "Uma inferência de modelo não pode ser fato verificado.",
  real_person_uncertain:
    "Opiniões sobre pessoas reais exigem cautela adicional.",
  defamation_blocked: "A descrição foi recusada por segurança.",
  invalid_status: "O status solicitado não é válido.",
  evidence_not_found: "A evidência não foi encontrada.",
  evidence_not_active: "A evidência já foi substituída.",
  invalid_subject: "O assunto informado não é válido.",
  relationship_not_found: "O relacionamento não foi encontrado.",
  relationship_delta_limit: "A alteração do relacionamento excede o limite.",
  relationship_rate_limit:
    "O limite de alterações do relacionamento foi atingido.",
  manipulation_blocked: "A alteração do relacionamento foi recusada.",
  invalid_goal: "Os dados do objetivo não são válidos.",
  external_action_blocked:
    "Objetivos fictícios não podem pedir ações externas.",
  invalid_goal_budget: "Use prioridade de 0 a 100 e orçamento de 1 a 1000.",
  invalid_goal_schedule: "O prazo do objetivo não é válido.",
  goal_not_found: "O objetivo não foi encontrado.",
  goal_loop_blocked: "A dependência criaria um ciclo de objetivos.",
  invalid_transition: "Essa mudança de status não é permitida.",
  conversation_temporary_blocked:
    "Conversas entre agentes não podem iniciar ou ser alteradas em conversa temporária.",
  conversation_purpose_invalid: "Informe um propósito público válido.",
  conversation_budget_invalid:
    "Os limites da conversa pública não são válidos.",
  conversation_opt_in_required:
    "Os dois agentes precisam autorizar este propósito explicitamente.",
  conversation_participant_invalid: "O participante da conversa não é válido.",
  conversation_blocked_safe_mode:
    "O modo seguro bloqueia conversas autônomas entre agentes.",
  conversation_blocked_silent:
    "O modo silencioso bloqueia conversas entre agentes.",
  conversation_blocked_suspended:
    "Agentes suspensos não participam de conversas.",
  conversation_not_found: "A conversa pública não foi encontrada.",
  conversation_not_active: "A conversa pública já terminou.",
  conversation_not_completed:
    "A conversa precisa terminar antes de gerar candidatos.",
  conversation_turn_invalid: "O turno público não é válido.",
  conversation_turn_limit: "O limite de turnos da conversa foi atingido.",
  conversation_token_limit: "O limite de tokens da conversa foi atingido.",
  conversation_duration_limit: "O tempo máximo da conversa foi atingido.",
  conversation_candidate_invalid: "O candidato público não é válido.",
  candidate_not_found: "O candidato não foi encontrado.",
  candidate_already_decided: "O candidato já recebeu uma decisão.",
  heavy_generation_busy: "Outra geração pesada já está em andamento.",
  invalid_resource_status: "O status do recurso não é válido.",
  resource_job_not_found: "O trabalho de recurso não foi encontrado.",
};

type CognitiveCommandMap = {
  list_cognitive_opinions: {
    args: { agentId: string };
    response: CognitiveOpinion[];
  };
  list_cognitive_relationships: {
    args: { agentId: string };
    response: RelationshipState[];
  };
  list_cognitive_goals: {
    args: { agentId: string };
    response: CognitiveGoal[];
  };
  list_fictional_activities: {
    args: { agentId: string };
    response: FictionalActivity[];
  };
  start_fictional_activity: {
    args: FictionalActivityRequest;
    response: FictionalActivity;
  };
  update_fictional_activity_status: {
    args: FictionalActivityStatusRequest;
    response: FictionalActivity;
  };
  propose_cognitive_opinion: {
    args: OpinionCandidateRequest;
    response: CognitiveOpinion;
  };
  correct_cognitive_opinion_evidence: {
    args: OpinionEvidenceCorrectionRequest;
    response: CognitiveOpinion;
  };
  set_cognitive_opinion_status: {
    args: CognitiveOpinionStatusRequest;
    response: CognitiveOpinion;
  };
  recalculate_cognitive_opinion: {
    args: CognitiveOpinionRecalculationRequest;
    response: CognitiveOpinion;
  };
  propose_cognitive_relationship: {
    args: RelationshipCandidateRequest;
    response: RelationshipState;
  };
  reset_cognitive_relationship: {
    args: RelationshipResetRequest;
    response: RelationshipState;
  };
  rollback_cognitive_relationship: {
    args: RelationshipRollbackRequest;
    response: RelationshipState;
  };
  create_owner_cognitive_goal: {
    args: GoalRequest;
    response: CognitiveGoal;
  };
  propose_agent_cognitive_goal: {
    args: GoalRequest;
    response: CognitiveGoal;
  };
  approve_cognitive_goal: {
    args: CognitiveGoalApprovalRequest;
    response: CognitiveGoal;
  };
  update_cognitive_goal_status: {
    args: CognitiveGoalStatusRequest;
    response: CognitiveGoal;
  };
  start_agent_conversation: {
    args: ConversationStartRequest;
    response: AgentConversationSummary;
  };
  append_public_conversation_turn: {
    args: PublicConversationTurnRequest;
    response: AgentConversationInspection;
  };
  emit_cognitive_candidate: {
    args: CognitiveCandidateRequest;
    response: CognitiveCandidate;
  };
  reserve_heavy_generation: {
    args: HeavyGenerationRequest;
    response: CognitiveResourceJob;
  };
  complete_resource_job: {
    args: ResourceJobCompletionRequest;
    response: CognitiveResourceJob;
  };
  list_agent_conversation_policies: {
    args: { agentId: string };
    response: ConversationPolicy[];
  };
  set_agent_conversation_policy: {
    args: ConversationPolicyRequest;
    response: ConversationPolicy;
  };
  list_cognitive_conversations: {
    args: { agentId: string };
    response: AgentConversationSummary[];
  };
  inspect_agent_conversation: {
    args: { agentId: string; conversationId: string };
    response: AgentConversationInspection;
  };
  interrupt_agent_conversation: {
    args: ConversationInterruptRequest;
    response: AgentConversationSummary;
  };
  list_cognitive_candidates: {
    args: { agentId: string };
    response: CognitiveCandidate[];
  };
  reject_cognitive_candidate: {
    args: CognitiveCandidateRejectionRequest;
    response: CognitiveCandidate;
  };
};

type CognitiveCommandName = keyof CognitiveCommandMap;

function invokeCognitive<Command extends CognitiveCommandName>(
  command: Command,
  args: CognitiveCommandMap[Command]["args"],
): Promise<CognitiveCommandMap[Command]["response"]> {
  return invoke<CognitiveCommandMap[Command]["response"]>(command, args);
}

function CognitiveCorePanel({ agentId }: { agentId: string }) {
  const participantAgentId =
    agentId === "agt_astra_provisional"
      ? "agt_luma_provisional"
      : "agt_astra_provisional";
  const [opinions, setOpinions] = useState<CognitiveOpinion[]>([]);
  const [relationships, setRelationships] = useState<RelationshipState[]>([]);
  const [goals, setGoals] = useState<CognitiveGoal[]>([]);
  const [activities, setActivities] = useState<FictionalActivity[]>([]);
  const [activityType, setActivityType] = useState("fictional-reading");
  const [conversationPolicies, setConversationPolicies] = useState<
    ConversationPolicy[]
  >([]);
  const [conversations, setConversations] = useState<
    AgentConversationSummary[]
  >([]);
  const [candidates, setCandidates] = useState<CognitiveCandidate[]>([]);
  const [selectedConversation, setSelectedConversation] =
    useState<AgentConversationInspection | null>(null);
  const [selectedConversationId, setSelectedConversationId] = useState<
    string | null
  >(null);
  const [conversationTurnSpeaker, setConversationTurnSpeaker] =
    useState(agentId);
  const [conversationTurnContent, setConversationTurnContent] = useState("");
  const [resourcePriority, setResourcePriority] = useState("50");
  const [resourceBudgetUnits, setResourceBudgetUnits] = useState("1");
  const [resourceJob, setResourceJob] = useState<CognitiveResourceJob | null>(
    null,
  );
  const [opinionSubject, setOpinionSubject] = useState("");
  const [opinionStance, setOpinionStance] = useState("0");
  const [opinionClaim, setOpinionClaim] = useState("");
  const [opinionReason, setOpinionReason] = useState("");
  const [opinionActionReason, setOpinionActionReason] = useState("");
  const [correctionOpinionId, setCorrectionOpinionId] = useState<string | null>(
    null,
  );
  const [correctionClaim, setCorrectionClaim] = useState("");
  const [relationshipSubject, setRelationshipSubject] = useState("");
  const [relationshipTrust, setRelationshipTrust] = useState("0.05");
  const [relationshipReason, setRelationshipReason] = useState("");
  const [relationshipActionReason, setRelationshipActionReason] = useState("");
  const [goalTitle, setGoalTitle] = useState("");
  const [goalDescription, setGoalDescription] = useState("");
  const [goalPriority, setGoalPriority] = useState("50");
  const [goalBudget, setGoalBudget] = useState("10");
  const [conversationPurpose, setConversationPurpose] = useState(
    "planejamento fictício",
  );
  const [conversationOptedIn, setConversationOptedIn] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const loadVersion = useRef(0);
  const activeAgentId = useRef(agentId);

  const load = useCallback(async () => {
    const version = ++loadVersion.current;
    const [
      nextOpinions,
      nextRelationships,
      nextGoals,
      nextActivities,
      nextPolicies,
      nextConversations,
      nextCandidates,
    ] = await Promise.all([
      invokeCognitive("list_cognitive_opinions", { agentId }),
      invokeCognitive("list_cognitive_relationships", { agentId }),
      invokeCognitive("list_cognitive_goals", { agentId }),
      invokeCognitive("list_fictional_activities", { agentId }),
      invokeCognitive("list_agent_conversation_policies", { agentId }),
      invokeCognitive("list_cognitive_conversations", { agentId }),
      invokeCognitive("list_cognitive_candidates", { agentId }),
    ]);
    if (version !== loadVersion.current) return;
    setOpinions(nextOpinions);
    setRelationships(nextRelationships);
    setGoals(nextGoals);
    setActivities(Array.isArray(nextActivities) ? nextActivities : []);
    setConversationPolicies(Array.isArray(nextPolicies) ? nextPolicies : []);
    setConversations(Array.isArray(nextConversations) ? nextConversations : []);
    setCandidates(Array.isArray(nextCandidates) ? nextCandidates : []);
    setSelectedConversation(null);
  }, [agentId]);

  useEffect(() => {
    activeAgentId.current = agentId;
    setOpinions([]);
    setRelationships([]);
    setGoals([]);
    setActivities([]);
    setConversationPolicies([]);
    setConversations([]);
    setCandidates([]);
    setSelectedConversation(null);
    setSelectedConversationId(null);
    setConversationTurnSpeaker(agentId);
    setConversationTurnContent("");
    setResourceJob(null);
    setError(null);
    setSuccess(null);
    setLoading(true);
    void load()
      .catch(() => {
        if (activeAgentId.current === agentId) {
          setError("Não foi possível carregar o núcleo cognitivo.");
        }
      })
      .finally(() => {
        if (activeAgentId.current === agentId) setLoading(false);
      });
  }, [agentId, load]);

  async function runCognitive<Command extends CognitiveCommandName>(
    command: Command,
    args: CognitiveCommandMap[Command]["args"],
    message: string,
    onResult?: (response: CognitiveCommandMap[Command]["response"]) => void,
  ): Promise<boolean> {
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const response = await invokeCognitive(command, args);
      await load();
      onResult?.(response);
      if (activeAgentId.current === agentId) setSuccess(message);
      return true;
    } catch (cause) {
      setError(
        coreErrorCopy[String(cause)] || "A operação cognitiva foi recusada.",
      );
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function proposeOpinion() {
    const stance = Number(opinionStance);
    if (
      !opinionSubject.trim() ||
      !opinionClaim.trim() ||
      !opinionReason.trim()
    ) {
      setError("Informe assunto, evidência e motivo da opinião.");
      return;
    }
    if (!Number.isFinite(stance) || stance < -1 || stance > 1) {
      setError("Use uma posição entre -1 e 1.");
      return;
    }
    if (
      await runCognitive(
        "propose_cognitive_opinion",
        {
          agentId,
          subjectType: "topic",
          subjectRef: opinionSubject,
          stance,
          confidence: 0.8,
          sourceKind: "owner_testimony",
          classification: "verified_fact",
          claimKey: "owner_claim",
          claimValue: opinionClaim,
          sourceReference: null,
          attribution: null,
          reason: opinionReason,
          idempotencyKey: crypto.randomUUID(),
          temporaryChat: false,
        },
        "Opinião proposta.",
      )
    ) {
      setOpinionSubject("");
      setOpinionClaim("");
      setOpinionReason("");
    }
  }

  function selectOpinionCorrection(opinion: CognitiveOpinion) {
    const evidence = opinion.evidence.find((item) => item.status === "active");
    if (!evidence) {
      setError("A opinião não possui evidência ativa para corrigir.");
      return;
    }
    setCorrectionOpinionId(opinion.id);
    setCorrectionClaim(evidence.claimValue);
    setError(null);
    setSuccess(null);
  }

  async function correctOpinion() {
    const opinion = opinions.find((item) => item.id === correctionOpinionId);
    const evidence = opinion?.evidence.find((item) => item.status === "active");
    if (!evidence) {
      setError("A opinião não possui evidência ativa para corrigir.");
      return;
    }
    if (!correctionClaim.trim() || !opinionActionReason.trim()) {
      setError("Informe a nova evidência e o motivo da correção.");
      return;
    }
    if (
      await runCognitive(
        "correct_cognitive_opinion_evidence",
        {
          agentId,
          evidenceId: evidence.id,
          claimValue: correctionClaim.trim(),
          reason: opinionActionReason.trim(),
          idempotencyKey: crypto.randomUUID(),
          temporaryChat: false,
        },
        "Evidência corrigida.",
      )
    ) {
      setCorrectionOpinionId(null);
      setCorrectionClaim("");
    }
  }

  async function setOpinionStatus(opinion: CognitiveOpinion) {
    if (!opinionActionReason.trim()) {
      setError("Informe o motivo da ação de opinião.");
      return;
    }
    await runCognitive(
      "set_cognitive_opinion_status",
      {
        agentId,
        opinionId: opinion.id,
        status: "disputed",
        reason: opinionActionReason.trim(),
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Opinião marcada para revisão.",
    );
  }

  async function recalculateOpinion(opinion: CognitiveOpinion) {
    if (!opinionActionReason.trim()) {
      setError("Informe o motivo da ação de opinião.");
      return;
    }
    await runCognitive(
      "recalculate_cognitive_opinion",
      {
        agentId,
        opinionId: opinion.id,
        reason: opinionActionReason.trim(),
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Opinião recalculada.",
    );
  }

  async function proposeRelationship() {
    const trust = Number(relationshipTrust);
    if (!relationshipSubject.trim() || !relationshipReason.trim()) {
      setError("Informe assunto e motivo do relacionamento.");
      return;
    }
    if (!Number.isFinite(trust) || trust < -0.1 || trust > 0.1 || trust === 0) {
      setError("A confiança deve ficar entre -0,1 e 0,1, sem ser zero.");
      return;
    }
    if (
      await runCognitive(
        "propose_cognitive_relationship",
        {
          agentId,
          subjectType: "agent",
          subjectRef: relationshipSubject,
          deltas: {
            familiarity: 0,
            trust,
            affinity: 0,
            admiration: 0,
            irritation: 0,
            reliabilityExpectation: 0,
          },
          sourceKind: "owner_testimony",
          sourceReference: null,
          confidence: 0.8,
          reason: relationshipReason,
          idempotencyKey: crypto.randomUUID(),
          temporaryChat: false,
        },
        "Relacionamento atualizado.",
      )
    ) {
      setRelationshipSubject("");
      setRelationshipReason("");
    }
  }

  async function resetRelationship(relationship: RelationshipState) {
    if (!relationshipActionReason.trim()) {
      setError("Informe o motivo da redefinição do relacionamento.");
      return;
    }
    await runCognitive(
      "reset_cognitive_relationship",
      {
        agentId,
        relationshipId: relationship.id,
        reason: relationshipActionReason.trim(),
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Relacionamento redefinido.",
    );
  }

  async function rollbackRelationship(relationship: RelationshipState) {
    const event = relationship.events[0];
    if (!event || event.status !== "applied") {
      setError("Não há um último evento aplicado para reverter.");
      return;
    }
    await runCognitive(
      "rollback_cognitive_relationship",
      {
        agentId,
        eventId: event.eventId,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Último evento do relacionamento revertido.",
    );
  }

  async function saveGoal(origin: "owner" | "agent_proposal") {
    const priority = Number(goalPriority);
    const budgetUnits = Number(goalBudget);
    if (!goalTitle.trim() || !goalDescription.trim()) {
      setError("Informe título e descrição do objetivo.");
      return;
    }
    if (
      !Number.isInteger(priority) ||
      priority < 0 ||
      priority > 100 ||
      !Number.isInteger(budgetUnits) ||
      budgetUnits < 1 ||
      budgetUnits > 1000
    ) {
      setError("Use prioridade de 0 a 100 e orçamento de 1 a 1000.");
      return;
    }
    const command =
      origin === "owner"
        ? "create_owner_cognitive_goal"
        : "propose_agent_cognitive_goal";
    if (
      await runCognitive(
        command,
        {
          agentId,
          title: goalTitle,
          description: goalDescription,
          priority,
          budgetUnits,
          dueAt: null,
          expiresAt: null,
          parentGoalId: null,
          idempotencyKey: crypto.randomUUID(),
          temporaryChat: false,
        },
        origin === "owner"
          ? "Objetivo do Owner criado."
          : "Objetivo fictício proposto.",
      )
    ) {
      setGoalTitle("");
      setGoalDescription("");
    }
  }

  async function approveGoal(goal: CognitiveGoal) {
    await runCognitive(
      "approve_cognitive_goal",
      {
        agentId,
        goalId: goal.id,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Objetivo aprovado.",
    );
  }

  async function setGoalStatus(
    goal: CognitiveGoal,
    status: "active" | "suspended" | "completed" | "rejected",
  ) {
    await runCognitive(
      "update_cognitive_goal_status",
      {
        agentId,
        goalId: goal.id,
        status,
        completionEvidence:
          status === "completed" ? "Concluído em estado fictício" : null,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      status === "completed" ? "Objetivo concluído." : "Status atualizado.",
    );
  }

  async function startActivity(goal: CognitiveGoal) {
    await runCognitive(
      "start_fictional_activity",
      {
        agentId,
        goalId: goal.id,
        activityType: activityType.trim(),
        budgetUnits: 1,
        durationMs: 60_000,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Atividade fictícia iniciada.",
    );
  }

  async function setActivityStatus(
    activity: FictionalActivity,
    status: FictionalActivity["status"],
  ) {
    await runCognitive(
      "update_fictional_activity_status",
      {
        agentId,
        activityId: activity.id,
        status,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Status da atividade atualizado.",
    );
  }

  async function saveConversationPolicy() {
    const purpose = conversationPurpose.trim();
    if (!purpose) {
      setError("Informe o propósito público da conversa.");
      return;
    }
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      await Promise.all(
        [agentId, participantAgentId].map((policyAgentId) =>
          invokeCognitive("set_agent_conversation_policy", {
            agentId: policyAgentId,
            purpose,
            optedIn: conversationOptedIn,
            maxTurns: 12,
            maxTokens: 2048,
            maxDurationMs: 300000,
            maxRepetitions: 2,
            resourceBudget: 20,
            temporaryChat: false,
          }),
        ),
      );
      await load();
      setSuccess(
        conversationOptedIn
          ? "Opt-in público atualizado nos dois agentes."
          : "Opt-in público revogado nos dois agentes.",
      );
    } catch (cause) {
      setError(
        coreErrorCopy[String(cause)] || "A operação cognitiva foi recusada.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function startConversation() {
    const purpose = conversationPurpose.trim();
    if (!purpose) {
      setError("Informe o propósito público da conversa.");
      return;
    }
    await runCognitive(
      "start_agent_conversation",
      {
        initiatorAgentId: agentId,
        participantAgentId,
        purpose,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Conversa pública iniciada.",
      (conversation) => {
        setSelectedConversationId(conversation.id);
        setSelectedConversation(null);
        setResourceJob(null);
      },
    );
  }

  async function inspectConversation(conversation: AgentConversationSummary) {
    setError(null);
    try {
      const inspection = await invokeCognitive("inspect_agent_conversation", {
        agentId,
        conversationId: conversation.id,
      });
      setSelectedConversationId(conversation.id);
      setSelectedConversation(inspection);
      setResourceJob((current) =>
        current?.conversationId === conversation.id ? current : null,
      );
    } catch (cause) {
      setError(
        coreErrorCopy[String(cause)] ||
          "Não foi possível inspecionar a conversa pública.",
      );
    }
  }

  async function interruptConversation(conversation: AgentConversationSummary) {
    await runCognitive(
      "interrupt_agent_conversation",
      {
        agentId,
        conversationId: conversation.id,
        reason: "Interrupção solicitada pelo Owner",
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Conversa pública interrompida.",
      () => {
        setSelectedConversationId(null);
        setSelectedConversation(null);
        setResourceJob(null);
      },
    );
  }

  async function appendPublicTurn() {
    const content = conversationTurnContent.trim();
    if (!selectedConversationId) {
      setError("Selecione uma conversa pública antes de registrar um turno.");
      return;
    }
    if (!content) {
      setError("Informe o conteúdo do turno público.");
      return;
    }
    if (content.length > 4096) {
      setError("O turno público deve ter no máximo 4096 caracteres.");
      return;
    }
    await runCognitive(
      "append_public_conversation_turn",
      {
        agentId,
        conversationId: selectedConversationId,
        speakerAgentId: conversationTurnSpeaker,
        content,
        sourceKind: "owner",
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Turno público registrado.",
      (inspection) => {
        setSelectedConversationId(inspection.conversation.id);
        setSelectedConversation(inspection);
        setConversationTurnContent("");
      },
    );
  }

  async function reserveHeavyGeneration() {
    const priority = Number(resourcePriority);
    const budgetUnits = Number(resourceBudgetUnits);
    if (!selectedConversationId) {
      setError("Selecione uma conversa pública antes de reservar recurso.");
      return;
    }
    if (
      !Number.isInteger(priority) ||
      priority < 0 ||
      priority > 100 ||
      !Number.isInteger(budgetUnits) ||
      budgetUnits < 1 ||
      budgetUnits > 100
    ) {
      setError("Use prioridade de 0 a 100 e recurso de 1 a 100.");
      return;
    }
    await runCognitive(
      "reserve_heavy_generation",
      {
        agentId,
        conversationId: selectedConversationId,
        priority,
        budgetUnits,
        idempotencyKey: crypto.randomUUID(),
      },
      "Trabalho pesado reservado.",
      (job) => setResourceJob(job),
    );
  }

  async function completeHeavyGeneration() {
    if (!resourceJob || resourceJob.status !== "running") {
      setError("Não há trabalho pesado em andamento para concluir.");
      return;
    }
    await runCognitive(
      "complete_resource_job",
      {
        agentId,
        jobId: resourceJob.id,
        status: "completed",
        errorCode: null,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Trabalho pesado concluído.",
      (job) => setResourceJob(job),
    );
  }

  async function rejectCandidate(candidate: CognitiveCandidate) {
    await runCognitive(
      "reject_cognitive_candidate",
      {
        agentId,
        candidateId: candidate.id,
        idempotencyKey: crypto.randomUUID(),
        temporaryChat: false,
      },
      "Candidato rejeitado; nenhum estado cognitivo foi aplicado.",
    );
  }

  const selectedConversationSummary =
    conversations.find(
      (conversation) => conversation.id === selectedConversationId,
    ) ?? selectedConversation?.conversation;

  return (
    <section
      className="cognitive-core-panel"
      aria-label="Núcleo cognitivo 7B a 7E"
      data-layout="cognitive-forms"
    >
      <h3>Núcleo cognitivo</h3>
      <p>
        Opiniões, relacionamentos e objetivos são registros limitados e
        fictícios; nenhuma ação externa é executada.
      </p>
      {loading ? <p>Carregando opiniões, relações e objetivos…</p> : null}
      <section className="cognitive-feature" data-feature="opinions">
        <h4>Opiniões</h4>
        <ul>
          {opinions.map((opinion) => (
            <li key={opinion.id}>
              {opinion.subjectRef}: posição {opinion.stance.toFixed(2)},
              confiança {opinion.confidence.toFixed(2)} ({opinion.status}) —{" "}
              {opinion.evidence.find((item) => item.status === "active")
                ? `${opinion.evidence.find((item) => item.status === "active")!.claimValue} — fonte: ${opinion.evidence.find((item) => item.status === "active")!.sourceKind}/${opinion.evidence.find((item) => item.status === "active")!.sourceReference ?? "Owner"}, classificação: ${opinion.evidence.find((item) => item.status === "active")!.classification}, confiança: ${opinion.evidence.find((item) => item.status === "active")!.confidence.toFixed(2)}`
                : "sem evidência ativa"}
              <div className="cognitive-form-actions">
                {opinion.evidence.some((item) => item.status === "active") ? (
                  <button
                    type="button"
                    disabled={busy}
                    aria-label={`Corrigir evidência de ${opinion.subjectRef}`}
                    onClick={() => selectOpinionCorrection(opinion)}
                  >
                    Corrigir evidência
                  </button>
                ) : null}{" "}
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void setOpinionStatus(opinion)}
                >
                  Marcar opinião como disputada
                </button>{" "}
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void recalculateOpinion(opinion)}
                >
                  Recalcular opinião
                </button>
              </div>
            </li>
          ))}
        </ul>
        <div className="cognitive-form-grid">
          <label>
            Assunto da opinião
            <input
              value={opinionSubject}
              maxLength={128}
              onChange={(event) => setOpinionSubject(event.target.value)}
            />
          </label>
          <label>
            Posição (-1 a 1)
            <input
              type="number"
              min="-1"
              max="1"
              step="0.01"
              value={opinionStance}
              onChange={(event) => setOpinionStance(event.target.value)}
            />
          </label>
          <label>
            Evidência do Owner
            <textarea
              value={opinionClaim}
              maxLength={500}
              onChange={(event) => setOpinionClaim(event.target.value)}
            />
          </label>
          <label>
            Motivo da opinião
            <textarea
              value={opinionReason}
              maxLength={500}
              onChange={(event) => setOpinionReason(event.target.value)}
            />
          </label>
        </div>
        <div className="cognitive-form-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => void proposeOpinion()}
          >
            Propor opinião
          </button>
        </div>
        <div className="cognitive-form-grid">
          <label>
            Motivo da ação de opinião
            <textarea
              value={opinionActionReason}
              maxLength={500}
              onChange={(event) => setOpinionActionReason(event.target.value)}
            />
          </label>
        </div>
        {correctionOpinionId ? (
          <div className="cognitive-form-grid cognitive-wide">
            <p>Correção da evidência selecionada</p>
            <label>
              Nova evidência
              <textarea
                value={correctionClaim}
                maxLength={500}
                onChange={(event) => setCorrectionClaim(event.target.value)}
              />
            </label>
            <div className="cognitive-form-actions">
              <button
                type="button"
                disabled={busy}
                onClick={() => void correctOpinion()}
              >
                Confirmar correção da evidência
              </button>
            </div>
          </div>
        ) : null}
      </section>

      <section className="cognitive-feature" data-feature="relationships">
        <h4>Relacionamentos</h4>
        <ul>
          {relationships.map((relationship) => (
            <li key={relationship.id}>
              {relationship.subjectRef}: familiaridade{" "}
              {relationship.values.familiarity.toFixed(2)}, confiança{" "}
              {relationship.values.trust.toFixed(2)}, afinidade{" "}
              {relationship.values.affinity.toFixed(2)}, admiração{" "}
              {relationship.values.admiration.toFixed(2)}, irritação{" "}
              {relationship.values.irritation.toFixed(2)}, confiabilidade{" "}
              {relationship.values.reliabilityExpectation.toFixed(2)} —{" "}
              {relationship.events.length} evento(s)
              <div className="cognitive-form-actions">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void resetRelationship(relationship)}
                >
                  Redefinir relacionamento
                </button>{" "}
                {relationship.events[0]?.status === "applied" ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void rollbackRelationship(relationship)}
                  >
                    Reverter último evento
                  </button>
                ) : null}
              </div>
            </li>
          ))}
        </ul>
        <div className="cognitive-form-grid">
          <label>
            Assunto do relacionamento
            <input
              value={relationshipSubject}
              maxLength={128}
              onChange={(event) => setRelationshipSubject(event.target.value)}
            />
          </label>
          <label>
            Alteração de confiança (-0,1 a 0,1)
            <input
              type="number"
              min="-0.1"
              max="0.1"
              step="0.01"
              value={relationshipTrust}
              onChange={(event) => setRelationshipTrust(event.target.value)}
            />
          </label>
          <label>
            Motivo do relacionamento
            <textarea
              value={relationshipReason}
              maxLength={500}
              onChange={(event) => setRelationshipReason(event.target.value)}
            />
          </label>
        </div>
        <div className="cognitive-form-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => void proposeRelationship()}
          >
            Propor alteração de relacionamento
          </button>
        </div>
        <div className="cognitive-form-grid">
          <label>
            Motivo da redefinição do relacionamento
            <textarea
              value={relationshipActionReason}
              maxLength={500}
              onChange={(event) =>
                setRelationshipActionReason(event.target.value)
              }
            />
          </label>
        </div>
      </section>

      <section className="cognitive-feature" data-feature="goals">
        <h4>Objetivos fictícios</h4>
        <ul>
          {goals.map((goal) => (
            <li key={goal.id}>
              <strong>{goal.title}</strong> — {coreGoalStatusCopy[goal.status]}{" "}
              — orçamento {goal.budgetUnits}
              <p>{goal.description}</p>
              {goal.completionEvidence ? (
                <p>{goal.completionEvidence}</p>
              ) : null}
              <p>
                Origem: {goal.origin}; prazo: {goal.dueAt ?? "sem prazo"};
                expiração: {goal.expiresAt ?? "sem expiração"}; fictício: sim.
              </p>
              {goal.status === "proposed" ? (
                <>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void approveGoal(goal)}
                  >
                    Aprovar objetivo
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void setGoalStatus(goal, "rejected")}
                  >
                    Rejeitar objetivo
                  </button>
                </>
              ) : null}
              {goal.status === "active" ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void startActivity(goal)}
                >
                  Iniciar atividade fictícia
                </button>
              ) : null}
              {goal.status === "active" ? (
                <>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void setGoalStatus(goal, "completed")}
                  >
                    Concluir objetivo
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void setGoalStatus(goal, "suspended")}
                  >
                    Suspender objetivo
                  </button>
                </>
              ) : null}
              {goal.status === "suspended" ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void setGoalStatus(goal, "active")}
                >
                  Retomar objetivo
                </button>
              ) : null}
            </li>
          ))}
        </ul>
        <div className="cognitive-form-grid">
          <label>
            Tipo de atividade fictícia
            <input
              value={activityType}
              maxLength={64}
              onChange={(event) => setActivityType(event.target.value)}
            />
          </label>
        </div>
      </section>

      <section className="cognitive-feature" data-feature="activities">
        <h4>Atividades fictícias</h4>
        <ul>
          {activities.map((activity) => (
            <li key={activity.id}>
              {activity.activityType} — {activity.status} — orçamento{" "}
              {activity.budgetUnits}; somente simulação.{" "}
              {activity.status === "active" ? (
                <>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void setActivityStatus(activity, "paused")}
                  >
                    Pausar
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void setActivityStatus(activity, "completed")
                    }
                  >
                    Concluir
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void setActivityStatus(activity, "expired")}
                  >
                    Expirar
                  </button>
                </>
              ) : null}
              {activity.status === "paused" ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void setActivityStatus(activity, "active")}
                >
                  Retomar
                </button>
              ) : null}
              {activity.status !== "archived" ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void setActivityStatus(activity, "archived")}
                >
                  Arquivar
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      </section>

      <section className="cognitive-feature" data-feature="goal-form">
        <h4>Novo objetivo fictício</h4>
        <div className="cognitive-form-grid">
          <label>
            Título do objetivo
            <input
              value={goalTitle}
              maxLength={160}
              onChange={(event) => setGoalTitle(event.target.value)}
            />
          </label>
          <label>
            Descrição do objetivo
            <textarea
              value={goalDescription}
              maxLength={1000}
              onChange={(event) => setGoalDescription(event.target.value)}
            />
          </label>
          <label>
            Prioridade (0 a 100)
            <input
              type="number"
              min="0"
              max="100"
              value={goalPriority}
              onChange={(event) => setGoalPriority(event.target.value)}
            />
          </label>
          <label>
            Orçamento fictício (1 a 1000)
            <input
              type="number"
              min="1"
              max="1000"
              value={goalBudget}
              onChange={(event) => setGoalBudget(event.target.value)}
            />
          </label>
        </div>
        <div className="cognitive-form-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => void saveGoal("agent_proposal")}
          >
            Propor objetivo fictício
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void saveGoal("owner")}
          >
            Criar objetivo do Owner
          </button>
        </div>
      </section>
      <h4>Conversas públicas entre agentes</h4>
      <p>
        Cada propósito precisa de autorização explícita nos dois agentes.
        Somente turnos públicos são registrados; não existe canal privado.
      </p>
      <label>
        Propósito público
        <input
          value={conversationPurpose}
          maxLength={160}
          onChange={(event) => setConversationPurpose(event.target.value)}
        />
      </label>
      <label>
        <input
          type="checkbox"
          checked={conversationOptedIn}
          onChange={(event) => setConversationOptedIn(event.target.checked)}
        />{" "}
        Autorizar este propósito para {agentId} e {participantAgentId}
      </label>
      <button
        type="button"
        disabled={busy}
        onClick={() => void saveConversationPolicy()}
      >
        Salvar autorização pública
      </button>
      <button
        type="button"
        disabled={busy}
        onClick={() => void startConversation()}
      >
        Iniciar conversa pública
      </button>
      <ul aria-label="Políticas de conversas públicas">
        {conversationPolicies.map((policy) => (
          <li key={`${policy.agentId}:${policy.purpose}`}>
            {policy.agentId} — {policy.purpose} —{" "}
            {policy.optedIn ? "autorizado" : "revogado"} — até {policy.maxTurns}{" "}
            turnos, {policy.maxTokens} tokens, {policy.maxDurationMs} ms,{" "}
            {policy.maxRepetitions} repetições e {policy.resourceBudget}{" "}
            unidades de recurso
          </li>
        ))}
      </ul>
      <h5>Histórico de conversas públicas</h5>
      <ul aria-label="Conversas públicas">
        {conversations.map((conversation) => (
          <li key={conversation.id}>
            <strong>{conversation.purpose}</strong> — {conversation.status} —{" "}
            {conversation.turnCount}/{conversation.maxTurns} turnos —{" "}
            {conversation.tokenCount}/{conversation.maxTokens} tokens
            <div>
              <button
                type="button"
                disabled={busy}
                onClick={() => void inspectConversation(conversation)}
              >
                Inspecionar turnos públicos
              </button>{" "}
              {conversation.status === "active" ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void interruptConversation(conversation)}
                >
                  Interromper conversa pública
                </button>
              ) : null}
            </div>
          </li>
        ))}
      </ul>
      {selectedConversation ? (
        <div>
          <h5>
            Turnos públicos de {selectedConversation.conversation.purpose}
          </h5>
          <ul aria-label="Turnos públicos">
            {selectedConversation.turns.map((turn) => (
              <li key={turn.id}>
                {turn.turnIndex + 1}. {turn.speakerAgentId}: {turn.content}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {selectedConversationId ? (
        <div>
          <h5>Operações públicas da conversa selecionada</h5>
          <p>
            {selectedConversationSummary?.purpose ??
              "Conversa pública selecionada"}
            . Os limites são aplicados pelo núcleo Rust/SQLite.
          </p>
          <label>
            Agente do turno público
            <select
              value={conversationTurnSpeaker}
              onChange={(event) =>
                setConversationTurnSpeaker(event.target.value)
              }
            >
              <option value={agentId}>{agentId}</option>
              <option value={participantAgentId}>{participantAgentId}</option>
            </select>
          </label>
          <label>
            Turno público (máximo de 4096 caracteres)
            <textarea
              value={conversationTurnContent}
              maxLength={4096}
              onChange={(event) =>
                setConversationTurnContent(event.target.value)
              }
            />
          </label>
          <p>{conversationTurnContent.length}/4096 caracteres</p>
          <button
            type="button"
            disabled={busy}
            onClick={() => void appendPublicTurn()}
          >
            Registrar turno público
          </button>
          <h5>Trabalho pesado limitado</h5>
          <p>
            Uma geração pesada por vez; as unidades também respeitam o orçamento
            acumulado desta conversa.
          </p>
          <label>
            Prioridade (0 a 100)
            <input
              type="number"
              min="0"
              max="100"
              value={resourcePriority}
              onChange={(event) => setResourcePriority(event.target.value)}
            />
          </label>
          <label>
            Unidades de recurso (1 a 100)
            <input
              type="number"
              min="1"
              max="100"
              value={resourceBudgetUnits}
              onChange={(event) => setResourceBudgetUnits(event.target.value)}
            />
          </label>
          <button
            type="button"
            disabled={busy}
            onClick={() => void reserveHeavyGeneration()}
          >
            Reservar geração pesada
          </button>
          {resourceJob ? (
            <p>
              Trabalho pesado {resourceJob.status} — {resourceJob.budgetUnits}{" "}
              unidades.
              {resourceJob.status === "running" ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void completeHeavyGeneration()}
                >
                  Concluir trabalho pesado
                </button>
              ) : null}
            </p>
          ) : null}
        </div>
      ) : null}
      <h5>Candidatos cognitivos pendentes</h5>
      <p>
        Candidatos continuam pendentes até decisão do Owner e não alteram o
        estado cognitivo automaticamente.
      </p>
      <ul aria-label="Candidatos cognitivos">
        {candidates.map((candidate) => (
          <li key={candidate.id}>
            {candidate.candidateKind} — {candidate.status} —{" "}
            {candidate.candidateJson}
            {candidate.status === "pending" ? (
              <button
                type="button"
                disabled={busy}
                onClick={() => void rejectCandidate(candidate)}
              >
                Rejeitar candidato
              </button>
            ) : null}
          </li>
        ))}
      </ul>
      {error ? <p role="alert">{coreErrorCopy[error] || error}</p> : null}
      {success ? <p role="status">{success}</p> : null}
      <p>
        Atividades fictícias ainda não estão implementadas neste checkpoint.
      </p>
    </section>
  );
}

export function PixelDocumentEditor({ agentId }: { agentId: string }) {
  const [source, setSource] = useState("");
  const [activeLayerId, setActiveLayerId] = useState("body");
  const [error, setError] = useState<string | null>(null);
  const [color, setColor] = useState("#57d8bd");
  const [tool, setTool] = useState<
    "pencil" | "eraser" | "fill" | "eyedropper" | "select"
  >("pencil");
  const [mirror, setMirror] = useState(false);
  const [selection, setSelection] = useState<PixelSelection | null>(null);
  const [pendingLayerDeletion, setPendingLayerDeletion] = useState<
    PixelDocument["layers"][number] | null
  >(null);
  const [zoom, setZoom] = useState(4);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const importRef = useRef<HTMLInputElement>(null);
  const undoRef = useRef<string[]>([]);
  const redoRef = useRef<string[]>([]);
  const strokeRef = useRef<{ x: number; y: number } | null>(null);
  const selectionAnchorRef = useRef<{ x: number; y: number } | null>(null);
  const document = parsePixelDocument(source);
  const activeLayer =
    document?.layers.find((layer) => layer.id === activeLayerId) ??
    document?.layers[0];
  function replaceSource(next: string) {
    if (next === source) return;
    undoRef.current = [...undoRef.current.slice(-49), source];
    redoRef.current = [];
    setSource(next);
  }
  useEffect(() => {
    void invoke<string>("load_pixel_document", { agentId })
      .then((loaded) => {
        setSource(loaded);
        setActiveLayerId(parsePixelDocument(loaded)?.layers[0]?.id ?? "body");
      })
      .catch(() => setError("Não foi possível abrir a arte."));
  }, [agentId]);
  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const context = canvas.getContext("2d");
    if (context === null) return;
    context.fillStyle = "#10151c";
    context.fillRect(0, 0, 256, 256);
    try {
      for (const layer of document?.layers ?? [])
        for (const [x, y, pixelColor] of layer.pixels ?? []) {
          if (
            layer.visible !== false &&
            Number.isInteger(x) &&
            Number.isInteger(y) &&
            x >= 0 &&
            y >= 0 &&
            x < 64 &&
            y < 64
          ) {
            context.fillStyle = pixelColor;
            context.fillRect(x * 4, y * 4, 4, 4);
          }
        }
      if (selection !== null) {
        context.strokeStyle = "#57d8bd";
        context.lineWidth = 1;
        context.strokeRect(
          selection.x * 4 + 0.5,
          selection.y * 4 + 0.5,
          selection.width * 4 - 1,
          selection.height * 4 - 1,
        );
      }
    } catch {
      /* Invalid source stays editable in the raw document field. */
    }
  }, [document, selection, source]);
  function paint(event: React.PointerEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const x = Math.max(
      0,
      Math.min(
        63,
        Math.floor((event.nativeEvent.offsetX * 64) / canvas.clientWidth),
      ),
    );
    const y = Math.max(
      0,
      Math.min(
        63,
        Math.floor((event.nativeEvent.offsetY * 64) / canvas.clientHeight),
      ),
    );
    if (tool === "select") {
      const anchor = selectionAnchorRef.current ?? { x, y };
      selectionAnchorRef.current = anchor;
      setSelection(selectionRectangle(anchor.x, anchor.y, x, y));
      return;
    }
    try {
      if (
        document === null ||
        activeLayer === undefined ||
        activeLayer.locked
      ) {
        throw new Error("missing layer");
      }
      const layer = activeLayer;
      const existingColor = layer.pixels?.find(
        ([pixelX, pixelY]) => pixelX === x && pixelY === y,
      )?.[2];
      if (tool === "eyedropper") {
        setColor(existingColor ?? "#10151c");
        return;
      }
      if (tool === "fill") {
        replaceSource(
          JSON.stringify(
            updatePixelLayer(document, layer.id, (current) =>
              floodFillLayer(current, x, y, color),
            ),
          ),
        );
        setError(null);
        return;
      }
      const previous = strokeRef.current ?? { x, y };
      const nextLayer = paintPixelLayer(
        layer,
        previous.x,
        previous.y,
        x,
        y,
        tool === "pencil" ? "pencil" : "eraser",
        color,
        mirror,
      );
      strokeRef.current = { x, y };
      replaceSource(
        JSON.stringify(updatePixelLayer(document, layer.id, () => nextLayer)),
      );
      setError(null);
    } catch {
      setError("Arte inválida: use uma camada com pixels.");
    }
  }
  async function save() {
    try {
      await invoke("save_pixel_document", { agentId, sourceJson: source });
      setError(null);
    } catch {
      setError("A arte precisa ter camadas e pontos de encaixe válidos.");
    }
  }
  function updateLayers(next: PixelDocument, nextActive = activeLayerId) {
    setActiveLayerId(nextActive);
    replaceSource(JSON.stringify(next));
  }
  function confirmLayerDeletion() {
    if (document === null || pendingLayerDeletion === null) return;
    const layers = document.layers.filter(
      (current) => current.id !== pendingLayerDeletion.id,
    );
    updateLayers({ ...document, layers }, layers[0]?.id ?? "body");
    setPendingLayerDeletion(null);
  }
  function moveSelection(dx: number, dy: number) {
    if (
      document === null ||
      activeLayer === undefined ||
      selection === null ||
      activeLayer.locked
    )
      return;
    const inside = (x: number, y: number) =>
      x >= selection.x &&
      x < selection.x + selection.width &&
      y >= selection.y &&
      y < selection.y + selection.height;
    const moved = activeLayer.pixels.flatMap(([x, y, pixelColor]) => {
      if (!inside(x, y))
        return [[x, y, pixelColor] as [number, number, string]];
      const nextX = x + dx;
      const nextY = y + dy;
      return nextX >= 0 && nextX < 64 && nextY >= 0 && nextY < 64
        ? [[nextX, nextY, pixelColor] as [number, number, string]]
        : [];
    });
    updateLayers(
      updatePixelLayer(document, activeLayer.id, (layer) => ({
        ...layer,
        pixels: moved,
      })),
    );
    setSelection((current) =>
      current === null
        ? null
        : {
            ...current,
            x: Math.max(0, Math.min(63 - current.width + 1, current.x + dx)),
            y: Math.max(0, Math.min(63 - current.height + 1, current.y + dy)),
          },
    );
  }
  async function importPng(file: File | undefined) {
    if (file === undefined) return;
    if (
      file.type !== "image/png" ||
      file.size > 1_000_000 ||
      document === null
    ) {
      setError("Use um PNG de até 1 MB.");
      return;
    }
    const url = URL.createObjectURL(file);
    try {
      const image = new Image();
      await new Promise<void>((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () => reject(new Error("invalid_png"));
        image.src = url;
      });
      const imported = globalThis.document.createElement("canvas");
      imported.width = 64;
      imported.height = 64;
      const context = imported.getContext("2d", { willReadFrequently: true });
      if (context === null) throw new Error("canvas_unavailable");
      context.drawImage(image, 0, 0, 64, 64);
      const pixels = Array.from(context.getImageData(0, 0, 64, 64).data).reduce<
        [number, number, string][]
      >((result, _, index, data) => {
        const alpha = data[index + 3] ?? 0;
        if (index % 4 !== 0 || alpha === 0) return result;
        const x = (index / 4) % 64;
        const y = Math.floor(index / 256);
        const color = rgbaToHex(
          data[index] ?? 0,
          data[index + 1] ?? 0,
          data[index + 2] ?? 0,
          alpha,
        );
        if (color !== null) result.push([x, y, color]);
        return result;
      }, []);
      const id = nextLayerId(document);
      updateLayers(
        {
          ...document,
          layers: [
            ...document.layers,
            { id, name: "PNG importado", visible: true, locked: false, pixels },
          ],
        },
        id,
      );
      setError(null);
    } catch {
      setError("Não foi possível importar este PNG.");
    } finally {
      URL.revokeObjectURL(url);
      if (importRef.current !== null) importRef.current.value = "";
    }
  }
  return (
    <details className="pixel-editor" open>
      <summary>Editor de pixel art (64×64)</summary>
      <div className="pixel-tools">
        <input
          type="color"
          value={color}
          onChange={(event) => setColor(event.target.value)}
          aria-label="Cor"
        />
        <button
          type="button"
          className={tool === "pencil" ? "active" : ""}
          onClick={() => setTool("pencil")}
        >
          Lápis
        </button>
        <button
          type="button"
          className={tool === "eraser" ? "active" : ""}
          onClick={() => setTool("eraser")}
        >
          Borracha
        </button>
        <button
          type="button"
          className={tool === "fill" ? "active" : ""}
          onClick={() => setTool("fill")}
        >
          Preencher
        </button>
        <button
          type="button"
          className={tool === "eyedropper" ? "active" : ""}
          onClick={() => setTool("eyedropper")}
        >
          Conta-gotas
        </button>
        <button
          type="button"
          className={tool === "select" ? "active" : ""}
          onClick={() => setTool("select")}
        >
          Selecionar
        </button>
        <label>
          <input
            type="checkbox"
            checked={mirror}
            onChange={(event) => setMirror(event.target.checked)}
          />{" "}
          Espelhar
        </label>
        <label>
          Zoom
          <select
            value={zoom}
            onChange={(event) => setZoom(Number(event.target.value))}
          >
            {[2, 4, 6, 8].map((value) => (
              <option key={value} value={value}>
                {value}×
              </option>
            ))}
          </select>
        </label>
        <input
          ref={importRef}
          type="file"
          accept="image/png"
          aria-label="Importar PNG"
          onChange={(event) => void importPng(event.currentTarget.files?.[0])}
        />
        <button
          type="button"
          disabled={undoRef.current.length === 0}
          onClick={() => {
            const previous = undoRef.current.pop();
            if (previous !== undefined) {
              redoRef.current.push(source);
              setSource(previous);
            }
          }}
        >
          Desfazer
        </button>
        <button
          type="button"
          disabled={redoRef.current.length === 0}
          onClick={() => {
            const next = redoRef.current.pop();
            if (next !== undefined) {
              undoRef.current.push(source);
              setSource(next);
            }
          }}
        >
          Refazer
        </button>
        <button
          type="button"
          disabled={selection === null || activeLayer?.locked}
          onClick={() => moveSelection(-1, 0)}
        >
          ←
        </button>
        <button
          type="button"
          disabled={selection === null || activeLayer?.locked}
          onClick={() => moveSelection(1, 0)}
        >
          →
        </button>
        <button
          type="button"
          disabled={selection === null || activeLayer?.locked}
          onClick={() => moveSelection(0, -1)}
        >
          ↑
        </button>
        <button
          type="button"
          disabled={selection === null || activeLayer?.locked}
          onClick={() => moveSelection(0, 1)}
        >
          ↓
        </button>
      </div>
      {document ? (
        <div className="pixel-layers" aria-label="Camadas">
          <div>
            <strong>Camadas</strong>
            <button
              type="button"
              onClick={() => {
                const id = nextLayerId(document);
                updateLayers(
                  {
                    ...document,
                    layers: [
                      ...document.layers,
                      {
                        id,
                        name: `Camada ${document.layers.length + 1}`,
                        visible: true,
                        locked: false,
                        pixels: [],
                      },
                    ],
                  },
                  id,
                );
              }}
            >
              Nova camada
            </button>
          </div>
          {document.layers.map((layer, index) => (
            <div
              key={layer.id}
              className={
                layer.id === activeLayer?.id
                  ? "pixel-layer active"
                  : "pixel-layer"
              }
            >
              <button type="button" onClick={() => setActiveLayerId(layer.id)}>
                {layer.name}
              </button>
              <input
                value={layer.name}
                aria-label={`Nome da camada ${layer.name}`}
                onChange={(event) =>
                  updateLayers(
                    updatePixelLayer(document, layer.id, (current) => ({
                      ...current,
                      name: event.target.value.slice(0, 64) || "Camada",
                    })),
                  )
                }
              />
              <button
                type="button"
                onClick={() =>
                  updateLayers(
                    updatePixelLayer(document, layer.id, (current) => ({
                      ...current,
                      visible: !current.visible,
                    })),
                  )
                }
              >
                {layer.visible ? "Ocultar" : "Mostrar"}
              </button>
              <button
                type="button"
                onClick={() =>
                  updateLayers(
                    updatePixelLayer(document, layer.id, (current) => ({
                      ...current,
                      locked: !current.locked,
                    })),
                  )
                }
              >
                {layer.locked ? "Desbloquear" : "Bloquear"}
              </button>
              <button
                type="button"
                disabled={index === 0}
                onClick={() => {
                  const layers = [...document.layers];
                  const previous = layers[index - 1];
                  const current = layers[index];
                  if (previous === undefined || current === undefined) return;
                  [layers[index - 1], layers[index]] = [current, previous];
                  updateLayers({ ...document, layers });
                }}
              >
                Subir
              </button>
              <button
                type="button"
                disabled={document.layers.length === 1}
                onClick={() => {
                  setPendingLayerDeletion(layer);
                }}
              >
                Excluir
              </button>
            </div>
          ))}
        </div>
      ) : null}
      {pendingLayerDeletion ? (
        <ConfirmDialog
          title="Excluir camada?"
          description={`A camada “${pendingLayerDeletion.name}” será removida da arte.`}
          confirmLabel="Excluir camada"
          onCancel={() => setPendingLayerDeletion(null)}
          onConfirm={confirmLayerDeletion}
        />
      ) : null}
      <canvas
        ref={canvasRef}
        width="256"
        height="256"
        className="pixel-canvas"
        style={{ width: `${64 * zoom}px`, height: `${64 * zoom}px` }}
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          strokeRef.current = null;
          selectionAnchorRef.current = null;
          paint(event);
        }}
        onPointerMove={(event) => {
          if (event.buttons === 1) paint(event);
        }}
        onPointerUp={(event) => {
          strokeRef.current = null;
          selectionAnchorRef.current = null;
          event.currentTarget.releasePointerCapture(event.pointerId);
        }}
        onPointerCancel={() => {
          strokeRef.current = null;
          selectionAnchorRef.current = null;
        }}
        aria-label="Grade de pixel art"
      />
      <details className="pixel-source">
        <summary>Avançado: documento da arte</summary>
        <textarea
          value={source}
          onChange={(event) => replaceSource(event.target.value)}
          aria-label="Documento de pixel art"
        />
      </details>
      <button type="button" onClick={() => void save()}>
        Salvar arte
      </button>
      {error ? <p role="alert">{error}</p> : null}
    </details>
  );
}

const toolLabels: Record<string, string> = {
  "workspace.inspect_scope": "Inspecionar área de arquivos fixture",
  "workspace.organize_files": "Organizar arquivos fixture",
  "workspace.inspect_local": "Inspecionar arquivos locais (raiz Owner)",
  "workspace.organize_local": "Mover arquivos locais (raiz Owner)",
  "calendar.list_events": "Listar eventos do calendário fixture",
  "calendar.create_event": "Criar evento no calendário fixture",
  "messaging.preview_message": "Pré-visualizar mensagem fixture",
  "messaging.send_message": "Enviar mensagem fixture",
};

const toolErrorLabels: Record<string, string> = {
  tools_blocked_temporary:
    "Ferramentas bloqueadas durante a conversa temporária.",
  tools_blocked_safe_mode: "Ferramentas bloqueadas pelo modo seguro.",
  tool_permission_denied: "A sessão não concedeu esta permissão.",
  tool_permission_invalid:
    "A combinação de ferramenta e permissão não é válida.",
  tool_scope_invalid: "A área fixture escolhida não é válida.",
  tool_input_invalid: "A entrada da ação não passou na validação.",
  tool_approval_required: "A aprovação explícita do Owner é necessária.",
  tool_confirmation_required: "A segunda confirmação explícita é necessária.",
  tool_action_rejected: "A ação foi recusada pelo Owner.",
  tool_action_not_executable: "A ação não está em um estado executável.",
  tool_session_cancelled: "A sessão de ferramentas está cancelada.",
  tool_compensation_unavailable:
    "Não há compensação disponível para esta ação.",
  workspace_root_unavailable: "A raiz local não está disponível.",
  workspace_root_invalid: "O caminho não é uma raiz local segura.",
  workspace_root_limit: "O limite de 64 raízes locais do Owner foi atingido.",
  workspace_path_unavailable: "O caminho relativo local não está disponível.",
  workspace_path_invalid: "O caminho relativo local não é seguro.",
  workspace_destination_exists:
    "O destino local já existe; nada foi sobrescrito.",
  workspace_move_failed: "A movimentação local falhou e foi registrada.",
  workspace_move_partial:
    "A movimentação local ficou parcial e foi registrada.",
  workspace_source_identity_unavailable:
    "A identidade da origem local não pôde ser verificada.",
  workspace_source_identity_mismatch:
    "A origem local mudou desde a prévia; nada foi movido.",
  tool_payload_invalid:
    "A resposta da ferramenta não passou no contrato seguro.",
};

const toolStatusLabels: Record<string, string> = {
  active: "ativa",
  cancelled: "cancelada",
  closed: "encerrada",
  previewed: "pré-visualizada",
  approved: "aprovada",
  confirmed: "confirmada",
  dry_run: "dry-run",
  executed: "executada pelo mock",
  failed: "falhou (estado registrado)",
  rejected: "recusada",
  compensated: "compensada",
};

const toolPermissionLabels: Record<string, string> = {
  preview: "prévia",
  execute_read_only: "execução somente leitura",
  execute_state_changing: "execução com mudança simulada",
};

const toolAuditLabels: Record<string, string> = {
  session_created: "sessão criada",
  session_cancelled: "sessão cancelada",
  action_previewed: "ação pré-visualizada",
  action_approved: "ação aprovada",
  action_rejected: "ação recusada",
  action_confirmed: "segunda confirmação registrada",
  action_dry_run: "dry-run executado",
  action_executed: "ação simulada",
  action_cancelled: "ação cancelada",
  action_compensated: "compensação registrada",
  action_failed: "ação local falhou",
};

function toolErrorMessage(error: unknown): string {
  const typed = parseCognitiveError(error);
  const code =
    typed?.code ??
    (typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "operation_unavailable");
  return (
    toolErrorLabels[code] ?? "A operação de ferramentas não está disponível."
  );
}

function parseToolPayload<T>(
  value: unknown,
  parser: (input: unknown) => T | null,
): T {
  const parsed = parser(value);
  if (parsed === null) throw new Error("tool_payload_invalid");
  return parsed;
}

export function ToolControls({
  agentId,
  temporaryChat,
  safeMode,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode: boolean;
}) {
  const [catalog, setCatalog] = useState<ToolManifest[]>([]);
  const [roots, setRoots] = useState<WorkspaceRoot[]>([]);
  const [rootPath, setRootPath] = useState("");
  const [selectedRootId, setSelectedRootId] = useState("");
  const [sessions, setSessions] = useState<ToolSession[]>([]);
  const [audit, setAudit] = useState<
    import("@aip/contracts").ToolAuditRecord[]
  >([]);
  const [selectedToolId, setSelectedToolId] = useState("");
  const [selectedSessionId, setSelectedSessionId] = useState("");
  const [scopeRef, setScopeRef] = useState("");
  const [allowPreview, setAllowPreview] = useState(true);
  const [allowExecute, setAllowExecute] = useState(true);
  const [action, setAction] = useState<ToolAction | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [relativePaths, setRelativePaths] = useState("inbox");
  const [moveFrom, setMoveFrom] = useState("inbox/entrada.txt");
  const [moveTo, setMoveTo] = useState("inbox/processado.txt");
  const [calendarTitle, setCalendarTitle] = useState("Revisão fixture");
  const [calendarDate, setCalendarDate] = useState("2026-08-20");
  const [calendarStart, setCalendarStart] = useState("10:00");
  const [calendarEnd, setCalendarEnd] = useState("11:00");
  const [recipient, setRecipient] = useState("fixture:recipient-owner");
  const [messageBody, setMessageBody] = useState("Mensagem de teste fixture");
  const [dryRun, setDryRun] = useState(false);

  const loadData = useCallback(async () => {
    const [rawCatalog, rawSessions, rawAudit, rawRoots] = await Promise.all([
      invoke<unknown>("list_tool_catalog"),
      invoke<unknown>("list_tool_sessions", { agentId }),
      invoke<unknown>("list_tool_audit", { agentId }),
      invoke<unknown>("list_workspace_roots"),
    ]);
    const nextCatalog = parseToolPayload(rawCatalog, parseToolCatalog);
    const nextSessions = parseToolPayload(rawSessions, parseToolSessions);
    const nextAudit = parseToolPayload(rawAudit, parseToolAudit);
    const nextRoots = parseToolPayload(rawRoots, parseWorkspaceRoots);
    setCatalog(nextCatalog);
    setSessions(nextSessions);
    setAudit(nextAudit);
    setRoots(nextRoots);
    setSelectedRootId((current) =>
      current && nextRoots.some((root) => root.id === current && root.enabled)
        ? current
        : (nextRoots.find((root) => root.enabled)?.id ?? ""),
    );
    setSelectedToolId((current) =>
      current &&
      nextCatalog.some(
        (manifest) =>
          manifest.toolId === current &&
          (manifest.scopeKind !== "workspace_root" ||
            nextRoots.some((root) => root.enabled)),
      )
        ? current
        : (nextCatalog[0]?.toolId ?? ""),
    );
    setSelectedSessionId((current) =>
      current && nextSessions.some((session) => session.id === current)
        ? current
        : (nextSessions[0]?.id ?? ""),
    );
  }, [agentId]);

  useEffect(() => {
    void loadData().catch((loadError: unknown) => {
      setError(toolErrorMessage(loadError));
    });
  }, [loadData]);

  const selectedManifest = catalog.find(
    (manifest) => manifest.toolId === selectedToolId,
  );
  const selectedSession = sessions.find(
    (session) => session.id === selectedSessionId,
  );
  const executePermission: ToolPermission =
    selectedManifest?.classification === "read_only"
      ? "execute_read_only"
      : "execute_state_changing";

  useEffect(() => {
    if (!selectedManifest) return;
    setScopeRef(
      selectedManifest.scopeKind === "workspace_root"
        ? selectedRootId
          ? `workspace_root:${selectedRootId}`
          : ""
        : `fixture:${selectedManifest.scopeKind}/owner`,
    );
    setAllowPreview(true);
    setAllowExecute(true);
  }, [selectedManifest, selectedRootId]);

  async function addRoot() {
    if (blocked || !rootPath.trim()) return;
    setBusy(true);
    setError(null);
    try {
      parseToolPayload(
        await invoke<unknown>("add_workspace_root", {
          agentId,
          path: rootPath.trim(),
          idempotencyKey: `workspace-root-${crypto.randomUUID()}`,
          temporaryChat,
        }),
        parseWorkspaceRoot,
      );
      setRootPath("");
      await loadData();
    } catch (rootError: unknown) {
      setError(toolErrorMessage(rootError));
    } finally {
      setBusy(false);
    }
  }

  async function disableRoot(rootId: string) {
    if (blocked) return;
    setBusy(true);
    setError(null);
    try {
      parseToolPayload(
        await invoke<unknown>("remove_workspace_root", {
          agentId,
          rootId,
          idempotencyKey: `workspace-root-disable-${crypto.randomUUID()}`,
          temporaryChat,
        }),
        parseWorkspaceRoot,
      );
      await loadData();
    } catch (rootError: unknown) {
      setError(toolErrorMessage(rootError));
    } finally {
      setBusy(false);
    }
  }

  function buildInput(): ToolActionInput | null {
    switch (selectedToolId) {
      case "workspace.inspect_scope":
      case "workspace.inspect_local":
        return {
          kind: "workspaceInspect",
          relativePaths: relativePaths
            .split(",")
            .map((path) => path.trim())
            .filter(Boolean),
        };
      case "workspace.organize_files":
      case "workspace.organize_local":
        return {
          kind: "workspaceOrganize",
          moves: [{ from: moveFrom, to: moveTo }],
        };
      case "calendar.list_events":
        return { kind: "calendarList", date: calendarDate };
      case "calendar.create_event":
        return {
          kind: "calendarCreate",
          title: calendarTitle,
          date: calendarDate,
          start: calendarStart,
          end: calendarEnd,
        };
      case "messaging.preview_message":
        return { kind: "messagingPreview", recipient, body: messageBody };
      case "messaging.send_message":
        return { kind: "messagingSend", recipient, body: messageBody };
      default:
        return null;
    }
  }

  async function createSession() {
    if (!selectedManifest || temporaryChat || safeMode) return;
    if (selectedManifest.scopeKind === "workspace_root" && !selectedRootId) {
      setError("Configure uma raiz local ativa antes de criar a sessão.");
      return;
    }
    const permissions: { toolId: string; permission: ToolPermission }[] = [];
    if (allowPreview) {
      permissions.push({
        toolId: selectedManifest.toolId,
        permission: "preview",
      });
    }
    if (allowExecute) {
      permissions.push({
        toolId: selectedManifest.toolId,
        permission: executePermission,
      });
    }
    if (permissions.length === 0) {
      setError("Escolha pelo menos uma permissão para a sessão.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next = parseToolPayload(
        await invoke<unknown>("create_tool_session", {
          agentId,
          scopeRef,
          permissions,
          idempotencyKey: `tool-session-${crypto.randomUUID()}`,
          temporaryChat: false,
        }),
        parseToolSession,
      );
      setSelectedSessionId(next.id);
      await loadData();
    } catch (createError: unknown) {
      setError(toolErrorMessage(createError));
    } finally {
      setBusy(false);
    }
  }

  async function previewAction() {
    const input = buildInput();
    if (!selectedManifest || !selectedSession || input === null) {
      setError("Escolha uma ferramenta e uma sessão ativa antes da prévia.");
      return;
    }
    if (selectedManifest.scopeKind === "workspace_root" && !selectedRootId) {
      setError("Configure uma raiz local ativa antes da prévia.");
      return;
    }
    if (temporaryChat || safeMode) return;
    setBusy(true);
    setError(null);
    try {
      const next = parseToolPayload(
        await invoke<unknown>("preview_tool_action", {
          agentId,
          sessionId: selectedSession.id,
          toolId: selectedManifest.toolId,
          input,
          dryRun,
          idempotencyKey: `tool-action-${crypto.randomUUID()}`,
          temporaryChat: false,
        }),
        parseToolAction,
      );
      setAction(next);
      await loadData();
    } catch (previewError: unknown) {
      setError(toolErrorMessage(previewError));
    } finally {
      setBusy(false);
    }
  }

  async function updateAction(command: string, args: Record<string, unknown>) {
    if (!action || temporaryChat || safeMode) return;
    setBusy(true);
    setError(null);
    try {
      const next = parseToolPayload(
        await invoke<unknown>(command, args),
        parseToolAction,
      );
      setAction(next);
      await loadData();
    } catch (actionError: unknown) {
      setError(toolErrorMessage(actionError));
    } finally {
      setBusy(false);
    }
  }

  async function cancelSession() {
    if (!selectedSession || temporaryChat || safeMode) return;
    setBusy(true);
    setError(null);
    try {
      parseToolPayload(
        await invoke<unknown>("cancel_tool_session", {
          agentId,
          sessionId: selectedSession.id,
          idempotencyKey: `tool-session-cancel-${crypto.randomUUID()}`,
          temporaryChat: false,
        }),
        parseToolSession,
      );
      setAction(null);
      await loadData();
    } catch (cancelError: unknown) {
      setError(toolErrorMessage(cancelError));
    } finally {
      setBusy(false);
    }
  }

  const blocked = temporaryChat || safeMode;
  const actionIsStateChanging = action?.classification === "state_changing";
  const actionIsPending =
    action !== null &&
    !["executed", "dry_run", "cancelled", "rejected", "compensated"].includes(
      action.status,
    );

  return (
    <section className="tool-controls" aria-label="Ferramentas supervisionadas">
      <h3>Ferramentas supervisionadas</h3>
      <p>
        Ferramentas fixture permanecem determinísticas. Inspeção local e
        movimentos limitados dentro de uma raiz Owner configurada têm efeito no
        host somente após prévia, aprovação e segunda confirmação; calendário e
        mensagens continuam mocks provider-neutral.
      </p>
      {temporaryChat ? (
        <p role="alert">Conversa temporária: ferramentas bloqueadas.</p>
      ) : null}
      {safeMode ? (
        <p role="alert">Modo seguro: ferramentas bloqueadas.</p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
      <section className="settings-card">
        <h4>Raízes locais do Owner</h4>
        <p>
          Somente caminhos escolhidos pelo Owner; a raiz é validada pelo Rust e
          não aparece na auditoria.
        </p>
        <input
          value={rootPath}
          onChange={(event) => setRootPath(event.target.value)}
          placeholder="Caminho local escolhido pelo Owner"
          disabled={busy || blocked}
        />
        <button
          type="button"
          onClick={() => void addRoot()}
          disabled={busy || blocked || !rootPath.trim()}
        >
          Adicionar raiz
        </button>
        <ul>
          {roots.map((root) => (
            <li key={root.id}>
              <code>{root.id}</code> — {root.enabled ? "ativa" : "desativada"}{" "}
              <button
                type="button"
                onClick={() => void disableRoot(root.id)}
                disabled={busy || blocked || !root.enabled}
              >
                Desativar
              </button>
            </li>
          ))}
        </ul>
        {roots.every((root) => !root.enabled) ? (
          <p role="status">
            Nenhuma raiz local ativa: as ferramentas fixture continuam
            disponíveis, mas as ferramentas locais aguardam uma raiz do Owner.
          </p>
        ) : null}
      </section>
      <section className="settings-card">
        <h4>Catálogo local v1</h4>
        <ul>
          {catalog.map((manifest) => (
            <li key={manifest.toolId}>
              <strong>
                {toolLabels[manifest.toolId] ?? "Ferramenta fixture"}
              </strong>{" "}
              <span>
                {manifest.adapterKind === "workspace_local"
                  ? "efeito local limitado no host"
                  : manifest.classification === "read_only"
                    ? "somente leitura"
                    : "altera estado somente no mock"}
                {manifest.requiresSecondConfirmation
                  ? "; exige segunda confirmação"
                  : ""}
              </span>
            </li>
          ))}
        </ul>
      </section>
      <fieldset disabled={busy || blocked}>
        <legend>Sessão com escopo explícito</legend>
        <label>
          Ferramenta
          <select
            value={selectedToolId}
            onChange={(event) => {
              setSelectedToolId(event.target.value);
              setAction(null);
            }}
          >
            {catalog.map((manifest) => (
              <option key={manifest.toolId} value={manifest.toolId}>
                {toolLabels[manifest.toolId] ?? manifest.toolId}
                {manifest.scopeKind === "workspace_root" &&
                !roots.some((root) => root.enabled)
                  ? " — raiz necessária"
                  : ""}
              </option>
            ))}
          </select>
        </label>
        <label>
          Escopo fixture
          <input
            value={scopeRef}
            onChange={(event) =>
              selectedManifest?.scopeKind !== "workspace_root" &&
              setScopeRef(event.target.value)
            }
            readOnly={selectedManifest?.scopeKind === "workspace_root"}
          />
        </label>
        {selectedManifest?.scopeKind === "workspace_root" ? (
          <>
            <label>
              Raiz local
              <select
                value={selectedRootId}
                onChange={(event) => setSelectedRootId(event.target.value)}
              >
                <option value="">Selecionar raiz ativa</option>
                {roots
                  .filter((root) => root.enabled)
                  .map((root) => (
                    <option key={root.id} value={root.id}>
                      {root.id}
                    </option>
                  ))}
              </select>
            </label>
            {!selectedRootId ? (
              <p role="status">
                Esta ferramenta local exige uma raiz local ativa do Owner para
                criar a sessão, gerar a prévia e solicitar aprovação.
              </p>
            ) : null}
          </>
        ) : null}
        <label>
          <input
            type="checkbox"
            checked={allowPreview}
            onChange={(event) => setAllowPreview(event.target.checked)}
          />{" "}
          Permitir prévia
        </label>
        <label>
          <input
            type="checkbox"
            checked={allowExecute}
            onChange={(event) => setAllowExecute(event.target.checked)}
          />{" "}
          Permitir{" "}
          {selectedManifest?.classification === "read_only"
            ? "execução somente leitura"
            : "execução com mudança simulada"}
        </label>
        <button type="button" onClick={() => void createSession()}>
          Criar sessão limitada
        </button>
      </fieldset>
      <label>
        Sessão atual
        <select
          value={selectedSessionId}
          onChange={(event) => {
            setSelectedSessionId(event.target.value);
            setAction(null);
          }}
          disabled={busy}
        >
          <option value="">Nenhuma sessão</option>
          {sessions.map((session) => (
            <option key={session.id} value={session.id}>
              {session.scopeRef} —{" "}
              {toolStatusLabels[session.status] ?? "estado desconhecido"}
            </option>
          ))}
        </select>
      </label>
      {selectedSession ? (
        <p>
          Permissões:{" "}
          {selectedSession.permissions
            .map(
              (permission) =>
                toolPermissionLabels[permission.permission] ??
                "permissão desconhecida",
            )
            .join(", ")}
          .
        </p>
      ) : null}
      <button
        type="button"
        disabled={busy || blocked || selectedSession?.status !== "active"}
        onClick={() => void cancelSession()}
      >
        Cancelar sessão
      </button>
      <fieldset
        disabled={busy || blocked || selectedSession?.status !== "active"}
      >
        <legend>Prévia da ação</legend>
        {selectedToolId === "workspace.inspect_scope" ||
        selectedToolId === "workspace.inspect_local" ? (
          <label>
            Entradas relativas, separadas por vírgula
            <input
              value={relativePaths}
              onChange={(event) => setRelativePaths(event.target.value)}
            />
          </label>
        ) : null}
        {selectedToolId === "workspace.organize_files" ||
        selectedToolId === "workspace.organize_local" ? (
          <>
            <label>
              Origem relativa
              <input
                value={moveFrom}
                onChange={(event) => setMoveFrom(event.target.value)}
              />
            </label>
            <label>
              Destino relativo
              <input
                value={moveTo}
                onChange={(event) => setMoveTo(event.target.value)}
              />
            </label>
          </>
        ) : null}
        {selectedToolId.startsWith("calendar.") ? (
          <>
            {selectedToolId === "calendar.create_event" ? (
              <label>
                Título
                <input
                  value={calendarTitle}
                  onChange={(event) => setCalendarTitle(event.target.value)}
                />
              </label>
            ) : null}
            <label>
              Data fixture
              <input
                type="date"
                value={calendarDate}
                onChange={(event) => setCalendarDate(event.target.value)}
              />
            </label>
            {selectedToolId === "calendar.create_event" ? (
              <>
                <label>
                  Início
                  <input
                    type="time"
                    value={calendarStart}
                    onChange={(event) => setCalendarStart(event.target.value)}
                  />
                </label>
                <label>
                  Fim
                  <input
                    type="time"
                    value={calendarEnd}
                    onChange={(event) => setCalendarEnd(event.target.value)}
                  />
                </label>
              </>
            ) : null}
          </>
        ) : null}
        {selectedToolId.startsWith("messaging.") ? (
          <>
            <label>
              Destinatário fixture
              <input
                value={recipient}
                onChange={(event) => setRecipient(event.target.value)}
              />
            </label>
            <label>
              Corpo da mensagem
              <textarea
                value={messageBody}
                onChange={(event) => setMessageBody(event.target.value)}
              />
            </label>
          </>
        ) : null}
        <label>
          <input
            type="checkbox"
            checked={dryRun}
            onChange={(event) => setDryRun(event.target.checked)}
          />{" "}
          Executar como dry-run
        </label>
        <button type="button" onClick={() => void previewAction()}>
          Gerar prévia limitada
        </button>
      </fieldset>
      {action ? (
        <section className="settings-card" aria-label="Ação selecionada">
          <h4>Ação selecionada</h4>
          <p>{action.summary}</p>
          <p>{action.exactEffect}</p>
          <p>Recursos afetados: {action.affectedResources.join(", ")}.</p>
          <p>
            Status: {toolStatusLabels[action.status] ?? "estado desconhecido"}.
          </p>
          <div className="message-actions">
            {actionIsStateChanging &&
            ["previewed", "approved", "confirmed"].includes(action.status) ? (
              <>
                <button
                  type="button"
                  disabled={busy || blocked}
                  onClick={() =>
                    void updateAction("approve_tool_action", {
                      agentId,
                      actionId: action.id,
                      approved: true,
                      idempotencyKey: `tool-approve-${crypto.randomUUID()}`,
                      temporaryChat: false,
                    })
                  }
                >
                  Aprovar como Owner
                </button>
                <button
                  type="button"
                  disabled={busy || blocked}
                  onClick={() =>
                    void updateAction("approve_tool_action", {
                      agentId,
                      actionId: action.id,
                      approved: false,
                      idempotencyKey: `tool-deny-${crypto.randomUUID()}`,
                      temporaryChat: false,
                    })
                  }
                >
                  Recusar ação
                </button>
              </>
            ) : null}
            {action.requiresSecondConfirmation &&
            action.status === "approved" ? (
              <button
                type="button"
                disabled={busy || blocked}
                onClick={() =>
                  void updateAction("confirm_tool_action", {
                    agentId,
                    actionId: action.id,
                    idempotencyKey: `tool-confirm-${crypto.randomUUID()}`,
                    temporaryChat: false,
                  })
                }
              >
                Confirmar segunda vez
              </button>
            ) : null}
            {actionIsPending ? (
              <button
                type="button"
                disabled={busy || blocked}
                onClick={() =>
                  void updateAction("execute_tool_action", {
                    agentId,
                    actionId: action.id,
                    dryRun: action.dryRun,
                    idempotencyKey: `tool-execute-${crypto.randomUUID()}`,
                    temporaryChat: false,
                  })
                }
              >
                {action?.toolId.endsWith("_local")
                  ? "Executar efeito local explicitamente"
                  : "Executar mock explicitamente"}
              </button>
            ) : null}
            {actionIsPending ? (
              <button
                type="button"
                disabled={busy || blocked}
                onClick={() =>
                  void updateAction("cancel_tool_action", {
                    agentId,
                    actionId: action.id,
                    idempotencyKey: `tool-cancel-${crypto.randomUUID()}`,
                    temporaryChat: false,
                  })
                }
              >
                Cancelar ação
              </button>
            ) : null}
            {action.status === "executed" && action.compensation?.available ? (
              <button
                type="button"
                disabled={busy || blocked}
                onClick={() =>
                  void updateAction("compensate_tool_action", {
                    agentId,
                    actionId: action.id,
                    idempotencyKey: `tool-compensate-${crypto.randomUUID()}`,
                    temporaryChat: false,
                  })
                }
              >
                Registrar compensação
              </button>
            ) : null}
          </div>
          {action.result ? (
            <div>
              <h5>
                {action.toolId.endsWith("_local")
                  ? "Saída não confiável da execução local"
                  : "Saída não confiável do mock"}
              </h5>
              <pre>{action.result.output}</pre>
              <p>Alteração no host: {action.result.changed ? "sim" : "não"}.</p>
            </div>
          ) : null}
        </section>
      ) : null}
      <section className="settings-card">
        <div className="message-actions">
          <h4>Auditoria recente</h4>
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void loadData().catch((loadError: unknown) =>
                setError(toolErrorMessage(loadError)),
              )
            }
          >
            Atualizar auditoria
          </button>
        </div>
        {audit.length === 0 ? (
          <p>Nenhum evento de ferramenta registrado para este agente.</p>
        ) : (
          <ul>
            {audit.slice(0, 20).map((record) => (
              <li key={record.id}>
                <strong>
                  {toolAuditLabels[record.event] ?? "evento de ferramenta"}
                </strong>
                : {record.summary}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  );
}

const extensionCapabilityLabels: Record<ExtensionCapability, string> = {
  agent_context: "contexto limitado do agente",
  tool_catalog: "catálogo de ferramentas",
  owner_review: "revisão do Owner",
};

const OWNER_USER_ID = "usr_owner_local";
const REPOSITORY_EXTENSION_EXAMPLE_ID = "fixture.notes";
const REPOSITORY_EXTENSION_EXAMPLE_INSTRUCTIONS: ExtensionInstruction[] = [
  {
    op: "emit_text",
    text: "Exemplo seguro do repositório.",
    echoInput: null,
  },
  { op: "yield" },
];

const extensionLifecycleLabels: Record<string, string> = {
  review_required: "aguarda revisão",
  approved: "aprovada, não ativada",
  active: "ativa",
  disabled: "desativada",
  rejected: "rejeitada",
  recovery_required: "requer recuperação",
};

const extensionProposalStatusLabels: Record<string, string> = {
  pending: "pendente",
  approved: "aprovada",
  rejected: "rejeitada",
  withdrawn: "retirada",
};

const extensionErrorLabels: Record<string, string> = {
  extensions_blocked_temporary:
    "Extensões bloqueadas durante a conversa temporária.",
  extensions_blocked_safe_mode: "Extensões bloqueadas pelo modo seguro.",
  extension_already_exists: "Já existe uma extensão com este identificador.",
  extension_not_found: "A extensão selecionada não foi encontrada.",
  extension_proposal_not_found: "A proposta selecionada não foi encontrada.",
  extension_manifest_invalid: "O manifesto não passou na validação.",
  extension_sdk_incompatible: "A versão do SDK não é compatível.",
  extension_id_invalid: "O identificador da extensão é inválido.",
  extension_version_invalid: "A versão deve seguir o formato x.y.z.",
  extension_text_invalid: "O texto do manifesto é inválido.",
  extension_fixture_invalid: "A referência fixture local é inválida.",
  extension_sandbox_invalid: "Somente a política metadata-only é permitida.",
  extension_admission_denied: "Somente fixtures locais são admitidos.",
  extension_untrusted_required: "A extensão precisa permanecer não confiável.",
  extension_capability_invalid: "A lista de capacidades é inválida.",
  extension_source_invalid: "A origem da extensão não é válida.",
  extension_review_required: "A proposta precisa de revisão do Owner.",
  extension_review_reason_required: "Informe o motivo da rejeição.",
  extension_permission_invalid: "As capacidades aprovadas não são válidas.",
  extension_permission_required: "Revise as permissões antes de ativar.",
  extension_update_requires_review:
    "A atualização exige nova revisão antes da ativação.",
  extension_rollback_unavailable:
    "Não há revisão aprovada disponível para rollback.",
  extension_owner_required: "Esta ação exige autorização explícita do Owner.",
  extension_proposal_self_review:
    "O agente que criou a proposta não pode revisá-la.",
  idempotency_conflict: "A operação conflita com uma solicitação anterior.",
  extension_payload_invalid:
    "A resposta da extensão não passou no contrato seguro.",
  extension_execution_denied:
    "A execução exige revisão, ativação e capacidades aprovadas.",
  extension_execution_busy: "Já existe uma execução de extensão em andamento.",
  extension_execution_cancelled: "A execução foi cancelada.",
  extension_execution_limit: "A execução atingiu o orçamento seguro.",
  extension_package_required: "Esta fixture metadata-only não é executável.",
};

function extensionErrorMessage(error: unknown): string {
  const typed = parseCognitiveError(error);
  const code =
    typed?.code ??
    (typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "operation_unavailable");
  return (
    extensionErrorLabels[code] ?? "A operação de extensões não está disponível."
  );
}

function parseExtensionPayload<T>(
  value: unknown,
  parser: (input: unknown) => T | null,
): T {
  const parsed = parser(value);
  if (parsed === null) throw new Error("extension_payload_invalid");
  return parsed;
}

export function ExtensionControls({
  agentId,
  temporaryChat,
  safeMode,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode: boolean;
}) {
  const [catalog, setCatalog] = useState<ExtensionCatalogEntry[]>([]);
  const [proposals, setProposals] = useState<ExtensionProposal[]>([]);
  const [audit, setAudit] = useState<ExtensionAuditRecord[]>([]);
  const [selectedExtensionId, setSelectedExtensionId] = useState("");
  const [selectedProposalId, setSelectedProposalId] = useState("");
  const [extensionId, setExtensionId] = useState("fixture.notes");
  const [extensionVersion, setExtensionVersion] = useState("1.0.0");
  const [extensionName, setExtensionName] = useState("Notas locais fixture");
  const [fixtureRef, setFixtureRef] = useState("fixture:extension/notes");
  const [sourceKind, setSourceKind] = useState<ExtensionSourceKind>(
    "administrator_selected",
  );
  const [capabilities, setCapabilities] = useState<ExtensionCapability[]>([
    "tool_catalog",
  ]);
  const [approvedCapabilities, setApprovedCapabilities] = useState<
    ExtensionCapability[]
  >([]);
  const [reviewReason, setReviewReason] = useState("");
  const [rollbackRevision, setRollbackRevision] = useState("1");
  const [disableReason, setDisableReason] = useState("Revisão manual do Owner");
  const [packageEnabled, setPackageEnabled] = useState(false);
  const [packageAgentContext, setPackageAgentContext] = useState(false);
  const [packageToolCatalog, setPackageToolCatalog] = useState(false);
  const [packageEchoInput, setPackageEchoInput] = useState(false);
  const [executionInput, setExecutionInput] = useState("");
  const [execution, setExecution] = useState<ExtensionExecutionResult | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const blocked = temporaryChat || safeMode;

  const loadData = useCallback(async () => {
    const [rawCatalog, rawProposals, rawAudit] = await Promise.all([
      invoke<unknown>("list_extension_catalog", { agentId }),
      invoke<unknown>("list_extension_proposals", { agentId }),
      invoke<unknown>("list_extension_audit", { agentId }),
    ]);
    const nextCatalog = parseExtensionPayload(
      rawCatalog,
      parseExtensionCatalog,
    );
    const nextProposals = parseExtensionPayload(
      rawProposals,
      parseExtensionProposals,
    );
    const nextAudit = parseExtensionPayload(rawAudit, parseExtensionAudit);
    setCatalog(nextCatalog);
    setProposals(nextProposals);
    setAudit(nextAudit);
    setSelectedExtensionId((current) =>
      current && nextCatalog.some((entry) => entry.extensionId === current)
        ? current
        : (nextCatalog[0]?.extensionId ?? ""),
    );
    setSelectedProposalId((current) =>
      current && nextProposals.some((proposal) => proposal.id === current)
        ? current
        : (nextProposals[0]?.id ?? ""),
    );
  }, [agentId]);

  useEffect(() => {
    void loadData().catch((loadError: unknown) => {
      setError(extensionErrorMessage(loadError));
    });
  }, [loadData]);

  const selectedCatalog = catalog.find(
    (entry) => entry.extensionId === selectedExtensionId,
  );
  const selectedProposal = proposals.find(
    (proposal) => proposal.id === selectedProposalId,
  );

  useEffect(() => {
    if (!selectedProposal) return;
    setApprovedCapabilities(selectedProposal.requestedCapabilities);
    setReviewReason("");
  }, [selectedProposal]);

  useEffect(() => {
    if (!selectedCatalog) return;
    setExtensionId(selectedCatalog.extensionId);
    setExtensionVersion(selectedCatalog.manifest.extensionVersion);
    setExtensionName(selectedCatalog.manifest.name);
    setFixtureRef(selectedCatalog.manifest.localFixtureRef ?? "");
    setSourceKind(selectedCatalog.sourceKind);
    setCapabilities(selectedCatalog.manifest.capabilities);
    setPackageEnabled(selectedCatalog.manifest.package != null);
    setRollbackRevision(
      String(Math.max(1, selectedCatalog.currentRevision - 1)),
    );
  }, [selectedCatalog]);

  async function buildPackage(): Promise<ExtensionManifest["package"]> {
    if (!packageEnabled) return null;
    const instructions: ExtensionInstruction[] = [];
    if (packageEchoInput) {
      instructions.push({ op: "emit_text", text: null, echoInput: true });
    } else {
      instructions.push({
        op: "emit_text",
        text: "Extensão A.I.P. executada.",
        echoInput: null,
      });
    }
    if (packageAgentContext) instructions.push({ op: "read_agent_context" });
    if (packageToolCatalog) instructions.push({ op: "list_tool_catalog" });
    instructions.push({ op: "yield" });
    const raw = await invoke<unknown>("build_extension_package", {
      instructions,
    });
    return parseExtensionPayload(raw, parseExtensionPackage);
  }

  async function importRepositoryExample() {
    if (
      blocked ||
      catalog.some(
        (entry) => entry.extensionId === REPOSITORY_EXTENSION_EXAMPLE_ID,
      )
    )
      return;
    setBusy(true);
    setError(null);
    try {
      const packageValue = parseExtensionPayload(
        await invoke<unknown>("build_extension_package", {
          instructions: REPOSITORY_EXTENSION_EXAMPLE_INSTRUCTIONS,
        }),
        parseExtensionPackage,
      );
      const manifest: ExtensionManifest = {
        extensionId: REPOSITORY_EXTENSION_EXAMPLE_ID,
        manifestVersion: 1,
        extensionVersion: "1.0.0",
        sdkVersion: "aip-extension-sdk/v1",
        name: "Notas locais do repositório",
        sandboxPolicy: "metadata_only",
        admissionPolicy: "local_fixture_only",
        capabilities: [],
        localFixtureRef: "fixture:extension/notes",
        untrusted: true,
        package: packageValue,
      };
      const request: ExtensionImportRequest = {
        agentId: agentId,
        ownerUserId: OWNER_USER_ID,
        manifestJson: JSON.stringify(manifest),
        idempotencyKey: `extension-example-${crypto.randomUUID()}`,
        temporaryChat,
      };
      const raw = await invoke<unknown>("import_extension_manifest", request);
      const next = parseExtensionPayload(raw, (value) => {
        const parsed = parseExtensionProposals([value]);
        return parsed?.[0] ?? null;
      });
      setSelectedProposalId(next.id);
      await loadData();
    } catch (importError: unknown) {
      setError(extensionErrorMessage(importError));
    } finally {
      setBusy(false);
    }
  }

  async function buildManifest(id = extensionId): Promise<ExtensionManifest> {
    return {
      extensionId: id.trim(),
      manifestVersion: 1,
      extensionVersion: extensionVersion.trim(),
      sdkVersion: "aip-extension-sdk/v1",
      name: extensionName.trim(),
      sandboxPolicy: "metadata_only",
      admissionPolicy: "local_fixture_only",
      capabilities,
      localFixtureRef: fixtureRef.trim() || null,
      untrusted: true,
      package: await buildPackage(),
    };
  }

  async function createProposal(kind: ExtensionSourceKind) {
    if (blocked) return;
    setBusy(true);
    setError(null);
    try {
      const manifest = await buildManifest();
      const raw =
        kind === "agent_created"
          ? await invoke<unknown>("create_agent_extension_proposal", {
              agentId,
              ownerUserId: OWNER_USER_ID,
              manifest,
              idempotencyKey: ["extension-agent-", crypto.randomUUID()].join(
                "",
              ),
              temporaryChat,
            })
          : await invoke<unknown>("create_extension_proposal", {
              agentId,
              ownerUserId: OWNER_USER_ID,
              sourceKind: kind,
              proposerAgentId: null,
              manifest,
              idempotencyKey: ["extension-owner-", crypto.randomUUID()].join(
                "",
              ),
              temporaryChat,
            });
      const next = parseExtensionPayload(raw, (value) => {
        const parsed = parseExtensionProposals([value]);
        return parsed?.[0] ?? null;
      });
      setSelectedProposalId(next.id);
      await loadData();
    } catch (createError: unknown) {
      setError(extensionErrorMessage(createError));
    } finally {
      setBusy(false);
    }
  }

  async function reviewProposal(approved: boolean) {
    if (!selectedProposal || blocked) return;
    if (!approved && !reviewReason.trim()) {
      setError("Informe o motivo para rejeitar a proposta.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke("review_extension_proposal", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proposalId: selectedProposal.id,
        approved,
        approvedCapabilities: approved ? approvedCapabilities : [],
        reason: reviewReason.trim() || null,
        idempotencyKey: ["extension-review-", crypto.randomUUID()].join(""),
        temporaryChat,
      });
      await loadData();
    } catch (reviewError: unknown) {
      setError(extensionErrorMessage(reviewError));
    } finally {
      setBusy(false);
    }
  }

  async function activateProposal() {
    if (!selectedProposal || blocked) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("activate_extension", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        extensionId: selectedProposal.extensionId,
        proposalId: selectedProposal.id,
        idempotencyKey: ["extension-activate-", crypto.randomUUID()].join(""),
        temporaryChat,
      });
      await loadData();
    } catch (activateError: unknown) {
      setError(extensionErrorMessage(activateError));
    } finally {
      setBusy(false);
    }
  }

  async function updateSelectedExtension() {
    if (!selectedCatalog || blocked) return;
    setBusy(true);
    setError(null);
    try {
      const raw = await invoke<unknown>("update_extension", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        extensionId: selectedCatalog.extensionId,
        sourceKind: selectedCatalog.sourceKind,
        proposerAgentId:
          selectedCatalog.sourceKind === "agent_created" ? agentId : null,
        manifest: await buildManifest(selectedCatalog.extensionId),
        idempotencyKey: ["extension-update-", crypto.randomUUID()].join(""),
        temporaryChat,
      });
      const next = parseExtensionPayload(raw, (value) => {
        const parsed = parseExtensionProposals([value]);
        return parsed?.[0] ?? null;
      });
      setSelectedProposalId(next.id);
      await loadData();
    } catch (updateError: unknown) {
      setError(extensionErrorMessage(updateError));
    } finally {
      setBusy(false);
    }
  }

  async function rollbackSelectedExtension() {
    if (!selectedCatalog || blocked) return;
    const targetRevision = Number(rollbackRevision);
    if (!Number.isInteger(targetRevision) || targetRevision < 1) {
      setError("Informe uma revisão anterior válida para rollback.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke("rollback_extension", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        extensionId: selectedCatalog.extensionId,
        targetRevision,
        idempotencyKey: ["extension-rollback-", crypto.randomUUID()].join(""),
        temporaryChat,
      });
      await loadData();
    } catch (rollbackError: unknown) {
      setError(extensionErrorMessage(rollbackError));
    } finally {
      setBusy(false);
    }
  }

  async function disableSelectedExtension() {
    if (!selectedCatalog || blocked) return;
    if (!disableReason.trim()) {
      setError("Informe o motivo da desativação.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke("disable_extension", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        extensionId: selectedCatalog.extensionId,
        reason: disableReason.trim(),
        idempotencyKey: ["extension-disable-", crypto.randomUUID()].join(""),
        temporaryChat,
      });
      await loadData();
    } catch (disableError: unknown) {
      setError(extensionErrorMessage(disableError));
    } finally {
      setBusy(false);
    }
  }

  async function executeSelectedExtension() {
    if (
      !selectedCatalog ||
      blocked ||
      selectedCatalog.lifecycle !== "active" ||
      !selectedCatalog.manifest.package
    )
      return;
    setBusy(true);
    setError(null);
    try {
      const raw = await invoke<unknown>("execute_extension", {
        request: {
          agentId,
          ownerUserId: OWNER_USER_ID,
          extensionId: selectedCatalog.extensionId,
          revision: selectedCatalog.activeRevision,
          packageHash: selectedCatalog.manifest.package.integritySha256,
          input: executionInput.slice(0, 4096),
          idempotencyKey: ["extension-execute-", crypto.randomUUID()].join(""),
          temporaryChat,
        },
      });
      setExecution(parseExtensionPayload(raw, parseExtensionExecutionResult));
    } catch (executeError: unknown) {
      setError(extensionErrorMessage(executeError));
    } finally {
      setBusy(false);
    }
  }

  async function cancelSelectedExecution() {
    if (!execution || blocked) return;
    setBusy(true);
    try {
      await invoke("cancel_extension_execution", {
        request: {
          agentId,
          ownerUserId: OWNER_USER_ID,
          executionId: execution.executionId,
        },
      });
      setExecution({
        ...execution,
        status: "cancelled",
        error: "extension_execution_cancelled",
      });
    } catch (cancelError: unknown) {
      setError(extensionErrorMessage(cancelError));
    } finally {
      setBusy(false);
    }
  }

  function toggleCapability(capability: ExtensionCapability) {
    setCapabilities((current) =>
      current.includes(capability)
        ? current.filter((value) => value !== capability)
        : [...current, capability],
    );
  }

  function toggleApprovedCapability(capability: ExtensionCapability) {
    setApprovedCapabilities((current) =>
      current.includes(capability)
        ? current.filter((value) => value !== capability)
        : [...current, capability],
    );
  }

  return (
    <section className="tool-controls" aria-label="Gerenciamento de extensões">
      <h3>Extensões locais</h3>
      <p>
        Catálogo privado de extensões não confiáveis. Pacotes executáveis são
        apenas programas declarativos fechados interpretados pelo Rust; não há
        acesso a rede, shell, arquivos do sistema, processos ou credenciais.
      </p>
      {temporaryChat ? (
        <p role="alert">
          Conversa temporária: alterações de extensões bloqueadas.
        </p>
      ) : null}
      {safeMode ? (
        <p role="alert">Modo seguro: alterações de extensões bloqueadas.</p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}

      <section className="settings-card">
        <h4>Exemplo seguro do repositório</h4>
        <p>
          {REPOSITORY_EXTENSION_EXAMPLE_ID} — fixture local, pacote declarativo
          fechado, não confiável e sem acesso a código nativo, shell, rede,
          arquivos ou credenciais. A importação cria uma proposta para revisão;
          não ativa nada automaticamente.
        </p>
        <button
          type="button"
          disabled={
            busy ||
            blocked ||
            catalog.some(
              (entry) => entry.extensionId === REPOSITORY_EXTENSION_EXAMPLE_ID,
            )
          }
          onClick={() => void importRepositoryExample()}
        >
          {catalog.some(
            (entry) => entry.extensionId === REPOSITORY_EXTENSION_EXAMPLE_ID,
          )
            ? "Exemplo já registrado"
            : "Adicionar exemplo seguro"}
        </button>
      </section>

      <section className="settings-card">
        <div className="message-actions">
          <h4>Catálogo privado</h4>
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void loadData().catch((loadError: unknown) =>
                setError(extensionErrorMessage(loadError)),
              )
            }
          >
            Atualizar catálogo e auditoria
          </button>
        </div>
        {catalog.length === 0 ? (
          <p>Nenhuma extensão local registrada.</p>
        ) : (
          <ul>
            {catalog.map((entry) => (
              <li key={entry.extensionId}>
                <strong>{entry.manifest.name}</strong> — {entry.extensionId} v
                {entry.manifest.extensionVersion};{" "}
                {extensionLifecycleLabels[entry.lifecycle] ??
                  "estado desconhecido"}
                ; {entry.untrusted ? "não confiável" : "inválida"}.
              </li>
            ))}
          </ul>
        )}
        <label>
          Extensão selecionada
          <select
            value={selectedExtensionId}
            onChange={(event) => setSelectedExtensionId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhuma extensão</option>
            {catalog.map((entry) => (
              <option key={entry.extensionId} value={entry.extensionId}>
                {entry.extensionId} —{" "}
                {extensionLifecycleLabels[entry.lifecycle]}
              </option>
            ))}
          </select>
        </label>
      </section>

      <section className="settings-card">
        <h4>Capacidade da extensão</h4>
        {selectedCatalog ? (
          <p>
            {selectedCatalog.manifest.package
              ? `Executável: ${selectedCatalog.manifest.package.format}; hash ${selectedCatalog.manifest.package.integritySha256}; revisão ${selectedCatalog.activeRevision ?? "não ativa"}.`
              : "Fixture metadata-only legada: não executável."}
          </p>
        ) : null}
        <fieldset disabled={busy || blocked}>
          <label>
            <input
              type="checkbox"
              checked={packageEnabled}
              onChange={(event) => setPackageEnabled(event.target.checked)}
            />{" "}
            Criar pacote declarativo executável
          </label>
          {packageEnabled ? (
            <>
              <label>
                <input
                  type="checkbox"
                  checked={packageAgentContext}
                  onChange={(event) =>
                    setPackageAgentContext(event.target.checked)
                  }
                />{" "}
                Ler identidade limitada do agente
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={packageToolCatalog}
                  onChange={(event) =>
                    setPackageToolCatalog(event.target.checked)
                  }
                />{" "}
                Listar catálogo limitado de ferramentas
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={packageEchoInput}
                  onChange={(event) =>
                    setPackageEchoInput(event.target.checked)
                  }
                />{" "}
                Repetir entrada limitada
              </label>
              <p>
                O Rust constrói e calcula o hash; a interface nunca calcula nem
                autoriza integridade.
              </p>
            </>
          ) : null}
        </fieldset>
      </section>

      <section className="settings-card">
        <h4>Propostas e revisão do Owner</h4>
        <label>
          Proposta selecionada
          <select
            value={selectedProposalId}
            onChange={(event) => setSelectedProposalId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhuma proposta</option>
            {proposals.map((proposal) => (
              <option key={proposal.id} value={proposal.id}>
                {proposal.extensionId} r{proposal.revision} —{" "}
                {extensionProposalStatusLabels[proposal.status]}
              </option>
            ))}
          </select>
        </label>
        {selectedProposal ? (
          <>
            <p>
              Origem:{" "}
              {selectedProposal.sourceKind === "agent_created"
                ? "proposta do agente"
                : "seleção do Owner"}
              ; revisão {selectedProposal.revision}; capacidades solicitadas:{" "}
              {selectedProposal.requestedCapabilities
                .map((capability) => extensionCapabilityLabels[capability])
                .join(", ") || "nenhuma"}
              .
            </p>
            <fieldset disabled={busy || blocked}>
              <legend>Permissões que serão aprovadas</legend>
              {selectedProposal.requestedCapabilities.map((capability) => (
                <label key={capability}>
                  <input
                    type="checkbox"
                    checked={approvedCapabilities.includes(capability)}
                    onChange={() => toggleApprovedCapability(capability)}
                  />{" "}
                  {extensionCapabilityLabels[capability]}
                </label>
              ))}
              <label>
                Motivo ou observação da revisão
                <textarea
                  value={reviewReason}
                  onChange={(event) => setReviewReason(event.target.value)}
                  placeholder="Obrigatório ao rejeitar"
                />
              </label>
              <div className="message-actions">
                <button type="button" onClick={() => void reviewProposal(true)}>
                  Aprovar proposta
                </button>
                <button
                  type="button"
                  onClick={() => void reviewProposal(false)}
                >
                  Rejeitar proposta
                </button>
              </div>
            </fieldset>
            <div className="message-actions">
              <button
                type="button"
                disabled={
                  busy || blocked || selectedProposal.status !== "approved"
                }
                onClick={() => void activateProposal()}
              >
                Ativar explicitamente
              </button>
            </div>
          </>
        ) : (
          <p>Nenhuma proposta pendente ou selecionada.</p>
        )}
      </section>

      <section className="settings-card">
        <h4>Registrar ou atualizar metadados</h4>
        <fieldset disabled={busy || blocked}>
          <label>
            Identificador
            <input
              value={extensionId}
              onChange={(event) => setExtensionId(event.target.value)}
            />
          </label>
          <label>
            Nome
            <input
              value={extensionName}
              onChange={(event) => setExtensionName(event.target.value)}
            />
          </label>
          <label>
            Versão x.y.z
            <input
              value={extensionVersion}
              onChange={(event) => setExtensionVersion(event.target.value)}
            />
          </label>
          <label>
            Referência fixture local
            <input
              value={fixtureRef}
              onChange={(event) => setFixtureRef(event.target.value)}
            />
          </label>
          <label>
            Origem da nova proposta
            <select
              value={sourceKind}
              onChange={(event) =>
                setSourceKind(event.target.value as ExtensionSourceKind)
              }
            >
              <option value="administrator_selected">
                Selecionada pelo Owner
              </option>
              <option value="agent_created">
                Proposta do agente (somente revisão)
              </option>
            </select>
          </label>
          <fieldset>
            <legend>Capacidades declaradas</legend>
            {(
              Object.keys(extensionCapabilityLabels) as ExtensionCapability[]
            ).map((capability) => (
              <label key={capability}>
                <input
                  type="checkbox"
                  checked={capabilities.includes(capability)}
                  onChange={() => toggleCapability(capability)}
                />{" "}
                {extensionCapabilityLabels[capability]}
              </label>
            ))}
          </fieldset>
          <div className="message-actions">
            <button
              type="button"
              onClick={() => void createProposal(sourceKind)}
            >
              {sourceKind === "agent_created"
                ? "Registrar proposta do agente"
                : "Registrar proposta do Owner"}
            </button>
            <button
              type="button"
              disabled={!selectedCatalog}
              onClick={() => void updateSelectedExtension()}
            >
              Enviar atualização para nova revisão
            </button>
          </div>
        </fieldset>
        <p>
          Atualizações desativam a revisão ativa e sempre voltam para revisão;
          ampliar capacidades não ativa permissões automaticamente.
        </p>
      </section>

      <section className="settings-card">
        <h4>Execução da revisão ativa</h4>
        <p>
          Somente uma extensão executável, revisada pelo Owner e explicitamente
          ativa pode executar.
        </p>
        <label>
          Entrada limitada
          <textarea
            value={executionInput}
            maxLength={4096}
            onChange={(event) => setExecutionInput(event.target.value)}
          />
        </label>
        <div className="message-actions">
          <button
            type="button"
            disabled={
              busy ||
              blocked ||
              !selectedCatalog?.manifest.package ||
              selectedCatalog.lifecycle !== "active"
            }
            onClick={() => void executeSelectedExtension()}
          >
            Executar revisão ativa
          </button>
          <button
            type="button"
            disabled={
              busy || blocked || !execution || execution.status !== "succeeded"
            }
            onClick={() => void cancelSelectedExecution()}
          >
            Cancelar execução registrada
          </button>
        </div>
        {execution ? (
          <p role="status">
            Execução {execution.executionId}: {execution.status}; passos{" "}
            {execution.steps}; saída: {execution.output ?? "(vazia)"}; erro:{" "}
            {execution.error ?? "nenhum"}
          </p>
        ) : (
          <p>Nenhuma execução solicitada.</p>
        )}
      </section>

      <section className="settings-card">
        <h4>Rollback e desativação explícita</h4>
        <fieldset disabled={busy || blocked || !selectedCatalog}>
          <label>
            Revisão aprovada alvo
            <input
              type="number"
              min="1"
              value={rollbackRevision}
              onChange={(event) => setRollbackRevision(event.target.value)}
            />
          </label>
          <label>
            Motivo da desativação
            <textarea
              value={disableReason}
              onChange={(event) => setDisableReason(event.target.value)}
            />
          </label>
          <div className="message-actions">
            <button
              type="button"
              onClick={() => void rollbackSelectedExtension()}
            >
              Fazer rollback
            </button>
            <button
              type="button"
              onClick={() => void disableSelectedExtension()}
            >
              Desativar extensão
            </button>
          </div>
        </fieldset>
      </section>

      <section className="settings-card">
        <h4>Auditoria recente</h4>
        {audit.length === 0 ? (
          <p>Nenhum evento de extensão registrado para este agente.</p>
        ) : (
          <ul>
            {audit.slice(0, 20).map((record) => (
              <li key={record.id}>
                <strong>{record.event}</strong>: {record.summary}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  );
}

const screenVisionStatusLabels: Record<string, string> = {
  active: "ativa",
  cancelled: "cancelada",
  closed: "encerrada",
  previewed: "prévia pronta",
  queued: "na fila",
  running: "em execução",
  completed: "concluída",
  failed: "falhou",
  cleaned: "limpa",
};

const screenVisionLifecycleLabels: Record<string, string> = {
  not_loaded: "modelo não carregado",
  loading: "carregando fixture",
  ready: "fixture pronta",
  running: "modelo em execução",
  unloaded: "modelo descarregado",
  unavailable: "modelo indisponível",
};

const screenVisionErrorLabels: Record<string, string> = {
  screen_vision_blocked_temporary:
    "Visão de tela bloqueada durante a conversa temporária.",
  screen_vision_blocked_safe_mode: "Visão de tela bloqueada pelo modo seguro.",
  screen_vision_blocked_suspended: "O agente está suspenso.",
  screen_vision_owner_required: "A confirmação do Owner local é necessária.",
  screen_vision_fixture_invalid: "A fixture de monitor selecionada é inválida.",
  screen_vision_permission_invalid:
    "A sessão precisa das permissões de fixture exigidas.",
  screen_vision_privacy_invalid:
    "Ative a exclusão de conteúdo sensível e a regra de redaction obrigatória.",
  screen_vision_quota_invalid: "A quota escolhida está fora do limite seguro.",
  screen_vision_session_limit: "O limite de sessões ativas foi atingido.",
  screen_vision_job_limit: "O limite de jobs desta sessão foi atingido.",
  screen_vision_session_cancelled: "A sessão de visão está cancelada.",
  screen_vision_confirmation_required:
    "A confirmação explícita do Owner é necessária.",
  screen_vision_job_invalid:
    "O job não está em um estado válido para esta ação.",
  screen_vision_resource_busy:
    "O recurso visual está ocupado; aguarde a limpeza do job atual.",
  screen_vision_payload_invalid:
    "A resposta de visão de tela não passou no contrato seguro.",
};

function screenVisionErrorMessage(error: unknown): string {
  const typed = parseCognitiveError(error);
  const code =
    typed?.code ??
    (typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "operation_unavailable");
  return (
    screenVisionErrorLabels[code] ??
    "A operação de visão de tela sintética não está disponível."
  );
}

function parseScreenVisionPayload<T>(
  value: unknown,
  parser: (input: unknown) => T | null,
): T {
  const parsed = parser(value);
  if (parsed === null) throw new Error("screen_vision_payload_invalid");
  return parsed;
}

export function ScreenVisionControls({
  agentId,
  temporaryChat,
  safeMode,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode: boolean;
}) {
  const [fixtures, setFixtures] = useState<ScreenVisionFixture[]>([]);
  const [sessions, setSessions] = useState<ScreenVisionSession[]>([]);
  const [jobs, setJobs] = useState<ScreenVisionJob[]>([]);
  const [audit, setAudit] = useState<ScreenVisionAuditRecord[]>([]);
  const [selectedFixtureId, setSelectedFixtureId] = useState("");
  const [selectedSessionId, setSelectedSessionId] = useState("");
  const [selectedJobId, setSelectedJobId] = useState("");
  const [allowCapture, setAllowCapture] = useState(true);
  const [allowAnalyze, setAllowAnalyze] = useState(true);
  const [excludeSensitiveContent, setExcludeSensitiveContent] = useState(true);
  const [excludeSensitiveRegions, setExcludeSensitiveRegions] = useState(true);
  const [excludeTextLikeRegions, setExcludeTextLikeRegions] = useState(false);
  const [maxJobs, setMaxJobs] = useState("4");
  const [maxDurationMs, setMaxDurationMs] = useState("5000");
  const [hypothesis, setHypothesis] = useState<ScreenVisionHypothesis | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    const [rawFixtures, rawSessions, rawJobs, rawAudit] = await Promise.all([
      invoke<unknown>("list_screen_vision_fixtures"),
      invoke<unknown>("list_screen_vision_sessions", { agentId }),
      invoke<unknown>("list_screen_vision_jobs", { agentId }),
      invoke<unknown>("list_screen_vision_audit", { agentId }),
    ]);
    const nextFixtures = parseScreenVisionPayload(
      rawFixtures,
      parseScreenVisionFixtures,
    );
    const nextSessions = parseScreenVisionPayload(
      rawSessions,
      parseScreenVisionSessions,
    );
    const nextJobs = parseScreenVisionPayload(rawJobs, parseScreenVisionJobs);
    const nextAudit = parseScreenVisionPayload(
      rawAudit,
      parseScreenVisionAudit,
    );
    setFixtures(nextFixtures);
    setSessions(nextSessions);
    setJobs(nextJobs);
    setAudit(nextAudit);
    setSelectedFixtureId((current) =>
      current && nextFixtures.some((fixture) => fixture.fixtureId === current)
        ? current
        : (nextFixtures[0]?.fixtureId ?? ""),
    );
    setSelectedSessionId((current) =>
      current && nextSessions.some((session) => session.id === current)
        ? current
        : (nextSessions[0]?.id ?? ""),
    );
    setSelectedJobId((current) =>
      current && nextJobs.some((job) => job.id === current)
        ? current
        : (nextJobs[0]?.id ?? ""),
    );
  }, [agentId]);

  useEffect(() => {
    void loadData().catch((loadError: unknown) => {
      setError(screenVisionErrorMessage(loadError));
    });
  }, [loadData]);

  const selectedFixture = fixtures.find(
    (fixture) => fixture.fixtureId === selectedFixtureId,
  );
  const selectedSession = sessions.find(
    (session) => session.id === selectedSessionId,
  );
  const selectedJob = jobs.find((job) => job.id === selectedJobId);
  const blocked = temporaryChat || safeMode;

  function buildPrivacy(): ScreenVisionPrivacyPolicy | null {
    if (!excludeSensitiveContent || !excludeSensitiveRegions) return null;
    return {
      excludeSensitiveContent: true,
      redactionRules: [
        { kind: "exclude_sensitive_regions", enabled: true },
        ...(excludeTextLikeRegions
          ? [{ kind: "exclude_text_like_regions" as const, enabled: true }]
          : []),
      ],
    };
  }

  async function createSession() {
    if (blocked || !selectedFixture) return;
    const permissions: ScreenVisionPermission[] = [];
    if (allowCapture) permissions.push("capture_fixture");
    if (allowAnalyze) permissions.push("analyze_fixture");
    if (permissions.length !== 2) {
      setError("Conceda as duas permissões para criar a sessão limitada.");
      return;
    }
    const privacy = buildPrivacy();
    if (privacy === null) {
      setError(
        "A exclusão de conteúdo sensível e a regra de regiões sensíveis são obrigatórias.",
      );
      return;
    }
    const jobsQuota = Number(maxJobs);
    const durationQuota = Number(maxDurationMs);
    if (
      !Number.isInteger(jobsQuota) ||
      !Number.isInteger(durationQuota) ||
      jobsQuota < 1 ||
      jobsQuota > 8 ||
      durationQuota < 100 ||
      durationQuota > 15_000
    ) {
      setError("Use de 1 a 8 jobs e duração entre 100 e 15000 ms.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next = parseScreenVisionPayload(
        await invoke<unknown>("create_screen_vision_session", {
          agentId,
          ownerUserId: OWNER_USER_ID,
          monitorId: selectedFixture.monitorId,
          fixtureId: selectedFixture.fixtureId,
          permissions,
          privacy,
          maxJobs: jobsQuota,
          maxDurationMs: durationQuota,
          idempotencyKey: `screen-session-${crypto.randomUUID()}`,
          temporaryChat,
        }),
        (value) => {
          const parsed = parseScreenVisionSessions([value]);
          return parsed?.[0] ?? null;
        },
      );
      setSelectedSessionId(next.id);
      setHypothesis(null);
      await loadData();
    } catch (createError: unknown) {
      setError(screenVisionErrorMessage(createError));
    } finally {
      setBusy(false);
    }
  }

  async function previewJob() {
    if (blocked || !selectedSession || selectedSession.status !== "active") {
      setError("Escolha uma sessão ativa antes de criar a prévia.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next = parseScreenVisionPayload(
        await invoke<unknown>("preview_screen_vision_job", {
          agentId,
          ownerUserId: OWNER_USER_ID,
          sessionId: selectedSession.id,
          idempotencyKey: `screen-preview-${crypto.randomUUID()}`,
          temporaryChat,
        }),
        (value) => {
          const parsed = parseScreenVisionJobs([value]);
          return parsed?.[0] ?? null;
        },
      );
      setSelectedJobId(next.id);
      setHypothesis(null);
      await loadData();
    } catch (previewError: unknown) {
      setError(screenVisionErrorMessage(previewError));
    } finally {
      setBusy(false);
    }
  }

  async function confirmJob() {
    if (blocked || !selectedJob || selectedJob.status !== "previewed") {
      setError("Selecione uma prévia pendente antes de confirmar.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = parseScreenVisionPayload(
        await invoke<unknown>("confirm_screen_vision_job", {
          agentId,
          ownerUserId: OWNER_USER_ID,
          jobId: selectedJob.id,
          confirmed: true,
          idempotencyKey: `screen-confirm-${crypto.randomUUID()}`,
          temporaryChat,
        }),
        parseScreenVisionAnalysisResult,
      );
      setSelectedJobId(result.job.id);
      setHypothesis(result.hypothesis);
      await loadData();
    } catch (confirmError: unknown) {
      setError(screenVisionErrorMessage(confirmError));
    } finally {
      setBusy(false);
    }
  }

  async function transitionJob(command: string, idempotencyPrefix: string) {
    if (blocked || !selectedJob || selectedJob.status === "cleaned") return;
    setBusy(true);
    setError(null);
    try {
      const next = parseScreenVisionPayload(
        await invoke<unknown>(command, {
          agentId,
          ownerUserId: OWNER_USER_ID,
          jobId: selectedJob.id,
          idempotencyKey: `${idempotencyPrefix}-${crypto.randomUUID()}`,
          temporaryChat,
        }),
        (value) => {
          const parsed = parseScreenVisionJobs([value]);
          return parsed?.[0] ?? null;
        },
      );
      setSelectedJobId(next.id);
      await loadData();
    } catch (transitionError: unknown) {
      setError(screenVisionErrorMessage(transitionError));
    } finally {
      setBusy(false);
    }
  }

  async function cancelSession() {
    if (blocked || !selectedSession || selectedSession.status !== "active")
      return;
    setBusy(true);
    setError(null);
    try {
      await invoke("cancel_screen_vision_session", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        sessionId: selectedSession.id,
        idempotencyKey: `screen-session-cancel-${crypto.randomUUID()}`,
        temporaryChat,
      });
      await loadData();
    } catch (cancelError: unknown) {
      setError(screenVisionErrorMessage(cancelError));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="tool-controls" aria-label="Visão de tela sob demanda">
      <h3>Visão de tela sob demanda</h3>
      <p>
        Displays reais do Windows aparecem quando disponíveis; a captura só
        ocorre após confirmação explícita do Owner, em memória e sem upload ou
        persistência padrão. Fixtures continuam determinísticas para testes.
      </p>
      <p>
        Owner confirmado: <code>{OWNER_USER_ID}</code>. O Rust valida essa
        identidade, as permissões, o modo e todo o ciclo de vida.
      </p>
      {temporaryChat ? (
        <p role="alert">Conversa temporária: alterações de visão bloqueadas.</p>
      ) : null}
      {safeMode ? (
        <p role="alert">Modo seguro: alterações de visão bloqueadas.</p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}

      <section className="settings-card">
        <div className="message-actions">
          <h4>Displays e fixtures disponíveis</h4>
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void loadData().catch((loadError: unknown) =>
                setError(screenVisionErrorMessage(loadError)),
              )
            }
          >
            Atualizar histórico
          </button>
        </div>
        {fixtures.length === 0 ? (
          <p>Nenhum display ou fixture disponível.</p>
        ) : (
          <>
            <label>
              Display ou fixture
              <select
                value={selectedFixtureId}
                onChange={(event) => setSelectedFixtureId(event.target.value)}
                disabled={busy}
              >
                {fixtures.map((fixture) => (
                  <option key={fixture.fixtureId} value={fixture.fixtureId}>
                    {fixture.displayName} — {fixture.width}×{fixture.height}
                  </option>
                ))}
              </select>
            </label>
            {selectedFixture ? (
              <p>
                {selectedFixture.synthetic
                  ? `Fixture sintética; monitor ${selectedFixture.monitorId}; metadata-only.`
                  : `Display real; monitor ${selectedFixture.monitorId}; captura sob demanda.`}{" "}
                Escala {selectedFixture.scale}.
              </p>
            ) : null}
          </>
        )}
      </section>

      <section className="settings-card">
        <h4>Nova sessão autorizada</h4>
        <fieldset disabled={busy || blocked || selectedFixture === undefined}>
          <legend>Permissões desta sessão</legend>
          <label>
            <input
              type="checkbox"
              checked={allowCapture}
              onChange={(event) => setAllowCapture(event.target.checked)}
            />{" "}
            Permitir captura do display selecionado
          </label>
          <label>
            <input
              type="checkbox"
              checked={allowAnalyze}
              onChange={(event) => setAllowAnalyze(event.target.checked)}
            />{" "}
            Permitir análise local limitada
          </label>
          <legend>Exclusão e redaction</legend>
          <label>
            <input
              type="checkbox"
              checked={excludeSensitiveContent}
              onChange={(event) =>
                setExcludeSensitiveContent(event.target.checked)
              }
            />{" "}
            Excluir conteúdo sensível (obrigatório)
          </label>
          <label>
            <input
              type="checkbox"
              checked={excludeSensitiveRegions}
              onChange={(event) =>
                setExcludeSensitiveRegions(event.target.checked)
              }
            />{" "}
            Excluir regiões sensíveis (obrigatório)
          </label>
          <label>
            <input
              type="checkbox"
              checked={excludeTextLikeRegions}
              onChange={(event) =>
                setExcludeTextLikeRegions(event.target.checked)
              }
            />{" "}
            Excluir regiões semelhantes a texto
          </label>
          <label>
            Quota de jobs por sessão (1–8)
            <input
              type="number"
              min="1"
              max="8"
              value={maxJobs}
              onChange={(event) => setMaxJobs(event.target.value)}
            />
          </label>
          <label>
            Duração máxima (100–15000 ms)
            <input
              type="number"
              min="100"
              max="15000"
              value={maxDurationMs}
              onChange={(event) => setMaxDurationMs(event.target.value)}
            />
          </label>
          <button
            type="button"
            disabled={busy || blocked || selectedFixture === undefined}
            onClick={() => void createSession()}
          >
            Criar sessão limitada
          </button>
        </fieldset>
      </section>

      <section className="settings-card">
        <h4>Sessões e prévias</h4>
        <label>
          Sessão atual
          <select
            value={selectedSessionId}
            onChange={(event) => setSelectedSessionId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhuma sessão</option>
            {sessions.map((session) => (
              <option key={session.id} value={session.id}>
                {session.monitorId} — {screenVisionStatusLabels[session.status]}
              </option>
            ))}
          </select>
        </label>
        {selectedSession ? (
          <>
            <p>
              Permissões: {selectedSession.permissions.join(", ")}; quota:{" "}
              {selectedSession.maxJobs} jobs / {selectedSession.maxDurationMs}{" "}
              ms; redaction obrigatória ativa.
            </p>
            <div className="message-actions">
              <button
                type="button"
                disabled={
                  busy || blocked || selectedSession.status !== "active"
                }
                onClick={() => void previewJob()}
              >
                Gerar prévia sem pixels
              </button>
              <button
                type="button"
                disabled={
                  busy || blocked || selectedSession.status !== "active"
                }
                onClick={() => void cancelSession()}
              >
                Cancelar sessão
              </button>
            </div>
          </>
        ) : (
          <p>Nenhuma sessão selecionada.</p>
        )}
      </section>

      <section className="settings-card">
        <h4>Jobs e confirmação explícita</h4>
        <label>
          Job atual
          <select
            value={selectedJobId}
            onChange={(event) => setSelectedJobId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhum job</option>
            {jobs.map((job) => (
              <option key={job.id} value={job.id}>
                {job.monitorId} — {screenVisionStatusLabels[job.status]}
              </option>
            ))}
          </select>
        </label>
        {selectedJob ? (
          <>
            <p>
              Status: {screenVisionStatusLabels[selectedJob.status]}; modelo:{" "}
              {screenVisionLifecycleLabels[selectedJob.modelLifecycle]};
              recurso: {selectedJob.resourceStatus}; cleanup:{" "}
              {selectedJob.cleanupStatus}.
            </p>
            <p>
              Prévia {selectedJob.preview.width}×{selectedJob.preview.height};
              confirmação necessária; bytes persistidos: não.
            </p>
            <div className="message-actions">
              <button
                type="button"
                disabled={busy || blocked || selectedJob.status !== "previewed"}
                onClick={() => void confirmJob()}
              >
                Confirmar e analisar agora
              </button>
              <button
                type="button"
                disabled={
                  busy ||
                  blocked ||
                  ["cleaned", "completed", "cancelled"].includes(
                    selectedJob.status,
                  )
                }
                onClick={() =>
                  void transitionJob(
                    "cancel_screen_vision_job",
                    "screen-cancel",
                  )
                }
              >
                Cancelar job
              </button>
              <button
                type="button"
                disabled={busy || blocked || selectedJob.status === "cleaned"}
                onClick={() =>
                  void transitionJob(
                    "cleanup_screen_vision_job",
                    "screen-cleanup",
                  )
                }
              >
                Limpar metadados agora
              </button>
            </div>
          </>
        ) : (
          <p>Nenhuma prévia selecionada.</p>
        )}
        {hypothesis ? (
          <section className="settings-card" aria-label="Hipótese incerta">
            <h4>Resultado incerto e não diagnóstico</h4>
            <p>{hypothesis.text}</p>
            <p>
              Confiança limitada: {hypothesis.confidence}%; fonte:{" "}
              {hypothesis.source}. Não é atributo sensível e não é salvo como
              memória visual.
            </p>
          </section>
        ) : null}
      </section>

      <section className="settings-card">
        <h4>Auditoria recente</h4>
        {audit.length === 0 ? (
          <p>Nenhum evento de visão sintética registrado para este agente.</p>
        ) : (
          <ul>
            {audit.slice(0, 20).map((record) => (
              <li key={record.id}>
                <strong>{record.event}</strong>: {record.summary}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  );
}

const companionDeviceStatusLabels: Record<string, string> = {
  pairing_requested: "aguardando confirmação",
  paired: "pareado",
  expired: "expirado",
  revoked: "revogado",
};

const companionQueueStatusLabels: Record<string, string> = {
  previewed: "prévia aguardando aprovação",
  queued: "aprovado, sem transporte",
  cancelled: "cancelado",
  failed: "falhou",
};

const companionErrorLabels: Record<string, string> = {
  companion_blocked_temporary:
    "Companion bloqueado durante a conversa temporária.",
  companion_blocked_safe_mode: "Companion bloqueado pelo modo seguro.",
  companion_blocked_suspended: "O agente suspenso não pode usar o companion.",
  companion_fixture_invalid: "A fixture Android local não é compatível.",
  companion_protocol_incompatible: "A versão do protocolo não é compatível.",
  companion_pairing_confirmation_required:
    "A confirmação explícita do Owner é necessária.",
  companion_pairing_required:
    "Pareie e confirme o dispositivo antes da sessão.",
  companion_authentication_failed: "A prova da sessão não foi autenticada.",
  companion_device_revoked: "O dispositivo foi revogado.",
  companion_replay_rejected:
    "A prova foi rejeitada por replay ou contador inválido.",
  companion_approval_required: "A aprovação explícita do Owner é necessária.",
  companion_queue_state_invalid:
    "O item não está em um estado válido para a ação.",
};

function companionErrorMessage(error: unknown): string {
  const typed = parseCognitiveError(error);
  const code =
    typed?.code ??
    (typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "operation_unavailable");
  return (
    companionErrorLabels[code] ??
    "A operação do companion local não está disponível."
  );
}

function parseCompanionPayload<T>(
  value: unknown,
  parser: (input: unknown) => T | null,
): T {
  const parsed = parser(value);
  if (parsed === null) throw new Error("companion_payload_invalid");
  return parsed;
}

function companionIdempotencyKey(prefix: string): string {
  return `${prefix}-${crypto.randomUUID()}`;
}

function companionProof(
  session: CompanionSession,
  purpose: string,
): CompanionSessionProof {
  return {
    sessionId: session.id,
    deviceId: session.deviceId,
    sessionNonceMetadata: session.sessionNonceMetadata,
    keyFingerprint: session.keyFingerprint,
    appVersion: session.appVersion,
    protocolVersion: session.protocolVersion,
    messageNonceMetadata: `fixture:message/${purpose}-${crypto.randomUUID()}`,
    replayCounter: session.lastReplayCounter + 1,
  };
}

export function CompanionControls({
  agentId,
  temporaryChat,
  safeMode,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode: boolean;
}) {
  const [devices, setDevices] = useState<CompanionDevice[]>([]);
  const [sessions, setSessions] = useState<CompanionSession[]>([]);
  const [queue, setQueue] = useState<CompanionQueueItem[]>([]);
  const [history, setHistory] = useState<CompanionHistoryRecord[]>([]);
  const [audit, setAudit] = useState<CompanionAuditRecord[]>([]);
  const [rotations, setRotations] = useState<CompanionKeyRotation[]>([]);
  const [revocations, setRevocations] = useState<CompanionRevocation[]>([]);
  const [transport, setTransport] = useState<GatewayTransportStatus>(
    EMPTY_GATEWAY_TRANSPORT_STATUS,
  );
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [selectedDeviceId, setSelectedDeviceId] = useState("");
  const [selectedQueueId, setSelectedQueueId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const blocked = temporaryChat || safeMode;

  const loadData = useCallback(async () => {
    const [
      rawDevices,
      rawSessions,
      rawQueue,
      rawHistory,
      rawAudit,
      rawRotations,
      rawRevocations,
      rawTransport,
    ] = await Promise.all([
      invoke<unknown>("list_companion_devices", { agentId }),
      invoke<unknown>("list_companion_sessions", { agentId }),
      invoke<unknown>("list_companion_queue", { agentId }),
      invoke<unknown>("list_companion_history", { agentId }),
      invoke<unknown>("list_companion_audit", { agentId }),
      invoke<unknown>("list_companion_key_rotations", { agentId }),
      invoke<unknown>("list_companion_revocations", { agentId }),
      invoke<unknown>("get_companion_transport_status"),
    ]);
    const nextDevices = parseCompanionPayload(
      rawDevices,
      parseCompanionDevices,
    );
    const nextSessions = parseCompanionPayload(
      rawSessions,
      parseCompanionSessions,
    );
    const nextQueue = parseCompanionPayload(rawQueue, parseCompanionQueue);
    const nextHistory = parseCompanionPayload(
      rawHistory,
      parseCompanionHistory,
    );
    const nextAudit = parseCompanionPayload(rawAudit, parseCompanionAudit);
    const nextRotations = parseCompanionPayload(
      rawRotations,
      parseCompanionKeyRotations,
    );
    const nextRevocations = parseCompanionPayload(
      rawRevocations,
      parseCompanionRevocations,
    );
    const nextTransport =
      parseGatewayTransportStatus(rawTransport) ??
      EMPTY_GATEWAY_TRANSPORT_STATUS;
    setDevices(nextDevices);
    setSessions(nextSessions);
    setQueue(nextQueue);
    setHistory(nextHistory);
    setAudit(nextAudit);
    setRotations(nextRotations);
    setRevocations(nextRevocations);
    setTransport(nextTransport);
    setSelectedDeviceId((current) =>
      nextDevices.some((device) => device.id === current)
        ? current
        : (nextDevices[0]?.id ?? ""),
    );
    setSelectedQueueId((current) =>
      nextQueue.some((item) => item.id === current)
        ? current
        : (nextQueue[0]?.id ?? ""),
    );
  }, [agentId]);

  useEffect(() => {
    void loadData().catch((loadError: unknown) =>
      setError(companionErrorMessage(loadError)),
    );
  }, [loadData]);

  const selectedDevice = devices.find(
    (device) => device.id === selectedDeviceId,
  );
  const selectedSession = selectedDevice
    ? sessions.find(
        (session) =>
          session.deviceId === selectedDevice.deviceId &&
          session.status === "connected",
      )
    : undefined;
  const selectedQueueItem = queue.find((item) => item.id === selectedQueueId);

  async function runMutation(operation: () => Promise<void>) {
    if (blocked || busy) return;
    setBusy(true);
    setError(null);
    try {
      await operation();
      await loadData();
    } catch (operationError: unknown) {
      setError(companionErrorMessage(operationError));
    } finally {
      setBusy(false);
    }
  }

  async function startTransport() {
    if (blocked || busy) return;
    setBusy(true);
    setError(null);
    setPairingCode(null);
    try {
      const raw = await invoke<unknown>("start_companion_transport", {
        agentId,
        ownerConfirmed: true,
        privateNetworkConfirmed: false,
        bindAddress: "127.0.0.1",
        port: 0,
        temporaryChat,
      });
      if (!isGatewayTransportStartResult(raw))
        throw new Error("companion_response_invalid");
      setTransport({
        enabled: true,
        endpoint: raw.endpoint,
        pairingAvailable: true,
      });
      setPairingCode(raw.pairingCode);
    } catch (operationError: unknown) {
      setError(companionErrorMessage(operationError));
    } finally {
      setBusy(false);
    }
  }

  async function stopTransport() {
    if (busy) return;
    setBusy(true);
    setError(null);
    setPairingCode(null);
    try {
      await invoke("stop_companion_transport");
      setTransport(EMPTY_GATEWAY_TRANSPORT_STATUS);
    } catch (operationError: unknown) {
      setError(companionErrorMessage(operationError));
    } finally {
      setBusy(false);
    }
  }

  async function connectSelectedDevice() {
    if (!selectedDevice) return;
    await runMutation(async () => {
      const nextCounter =
        Math.max(
          0,
          ...sessions
            .filter((session) => session.deviceId === selectedDevice.deviceId)
            .map((session) => session.lastReplayCounter),
        ) + 1;
      const rawSession = await invoke<unknown>("connect_companion_session", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        deviceId: selectedDevice.deviceId,
        appVersion: selectedDevice.appVersion,
        protocolVersion: selectedDevice.protocolVersion,
        fingerprint: selectedDevice.fingerprint,
        pairingNonceMetadata: selectedDevice.pairingNonceMetadata,
        messageNonceMetadata: `fixture:message/connect-${crypto.randomUUID()}`,
        replayCounter: nextCounter,
        idempotencyKey: companionIdempotencyKey("session-connect"),
        temporaryChat,
      });
      parseCompanionPayload(rawSession, parseCompanionSession);
    });
  }

  async function startPairing() {
    await runMutation(async () => {
      const rawDevice = await invoke<unknown>("start_companion_pairing", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        deviceId: COMPANION_FIXTURE_DEVICE_ID,
        platform: "android",
        appVersion: COMPANION_FIXTURE_APP_VERSION,
        protocolVersion: COMPANION_PROTOCOL_VERSION,
        fingerprint: COMPANION_FIXTURE_FINGERPRINT,
        pairingNonceMetadata: COMPANION_FIXTURE_PAIRING_NONCE,
        idempotencyKey: companionIdempotencyKey("pair-start"),
        temporaryChat,
      });
      const device = parseCompanionPayload(rawDevice, parseCompanionDevice);
      setSelectedDeviceId(device.id);
    });
  }

  async function confirmPairing() {
    if (!selectedDevice) return;
    await runMutation(async () => {
      const rawDevice = await invoke<unknown>("confirm_companion_pairing", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        deviceId: selectedDevice.deviceId,
        fingerprint: selectedDevice.fingerprint,
        pairingNonceMetadata: selectedDevice.pairingNonceMetadata,
        confirmed: true,
        idempotencyKey: companionIdempotencyKey("pair-confirm"),
        temporaryChat,
      });
      const device = parseCompanionPayload(rawDevice, parseCompanionDevice);
      setSelectedDeviceId(device.id);
      const rawSession = await invoke<unknown>("connect_companion_session", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        deviceId: device.deviceId,
        appVersion: device.appVersion,
        protocolVersion: device.protocolVersion,
        fingerprint: device.fingerprint,
        pairingNonceMetadata: device.pairingNonceMetadata,
        messageNonceMetadata: `fixture:message/connect-${crypto.randomUUID()}`,
        replayCounter: 1,
        idempotencyKey: companionIdempotencyKey("session-connect"),
        temporaryChat,
      });
      parseCompanionPayload(rawSession, parseCompanionSession);
    });
  }

  async function previewQueue() {
    if (!selectedSession) return;
    await runMutation(async () => {
      const rawItem = await invoke<unknown>("preview_companion_queue", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proof: companionProof(selectedSession, "queue-preview"),
        payload: {
          kind: "text",
          text: "Mensagem fixture do companion Android",
        },
        idempotencyKey: companionIdempotencyKey("queue-preview"),
        temporaryChat,
      });
      const item = parseCompanionPayload(rawItem, parseCompanionQueueItem);
      setSelectedQueueId(item.id);
    });
  }

  async function decideQueue(approved: boolean) {
    if (!selectedSession || !selectedQueueItem) return;
    await runMutation(async () => {
      const rawItem = await invoke<unknown>("approve_companion_queue", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proof: companionProof(selectedSession, "queue-approve"),
        queueId: selectedQueueItem.id,
        approved,
        idempotencyKey: companionIdempotencyKey("queue-approve"),
        temporaryChat,
      });
      parseCompanionPayload(rawItem, parseCompanionQueueItem);
    });
  }

  async function actOnQueue(
    command: "cancel_companion_queue" | "retry_companion_queue",
  ) {
    if (!selectedSession || !selectedQueueItem) return;
    await runMutation(async () => {
      const rawItem = await invoke<unknown>(command, {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proof: companionProof(
          selectedSession,
          command.replace("_companion_queue", ""),
        ),
        queueId: selectedQueueItem.id,
        idempotencyKey: companionIdempotencyKey(command),
        temporaryChat,
      });
      parseCompanionPayload(rawItem, parseCompanionQueueItem);
    });
  }

  async function rotateKey() {
    if (!selectedDevice) return;
    await runMutation(async () => {
      const rawRotation = await invoke<unknown>("rotate_companion_key", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        deviceId: selectedDevice.deviceId,
        reason: "Rotação solicitada pelo Owner no fixture local",
        idempotencyKey: companionIdempotencyKey("key-rotate"),
        temporaryChat,
      });
      parseCompanionPayload(rawRotation, parseCompanionKeyRotation);
    });
  }

  async function revokeDevice() {
    if (!selectedDevice) return;
    await runMutation(async () => {
      const rawRevocation = await invoke<unknown>("revoke_companion_device", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        deviceId: selectedDevice.deviceId,
        reason: "Revogação solicitada pelo Owner no fixture local",
        idempotencyKey: companionIdempotencyKey("device-revoke"),
        temporaryChat,
      });
      parseCompanionPayload(rawRevocation, parseCompanionRevocation);
    });
  }

  return (
    <section className="settings-card" aria-label="Companion Android local">
      <header>
        <h3>Companion Android local</h3>
        <p>
          Protocolo v{COMPANION_PROTOCOL_VERSION}; fixture sintética; somente
          comandos Tauri locais.
        </p>
      </header>
      <ul>
        <li>Somente local: sem rede, listener, relay ou conta externa.</li>
        <li>Somente metadados: bytes de mídia nunca são persistidos.</li>
        <li>Aprovação, prova de sessão, rotação e revogação são do Rust.</li>
      </ul>
      <section aria-label="Transporte de depuração do companion">
        <h4>Transporte local de depuração</h4>
        <p>
          Listener TCP autenticado em loopback, somente para validação do
          companion Android; não é relay de produção.
        </p>
        <p>
          Estado: <strong>{transport.enabled ? "ativo" : "parado"}</strong>;
          endpoint: <code>{transport.endpoint ?? "nenhum"}</code>; pairing:{" "}
          {transport.pairingAvailable ? "disponível" : "indisponível"}.
        </p>
        <div className="message-actions">
          <button
            type="button"
            disabled={busy || blocked || transport.enabled}
            onClick={() => void startTransport()}
          >
            Iniciar transporte local
          </button>
          <button
            type="button"
            disabled={busy || !transport.enabled}
            onClick={() => void stopTransport()}
          >
            Parar transporte local
          </button>
        </div>
        {pairingCode ? (
          <p role="alert">
            <strong>Código transitório:</strong> <code>{pairingCode}</code>. Não
            persista nem compartilhe.
          </p>
        ) : null}
      </section>
      {temporaryChat ? (
        <p role="alert">
          Conversa temporária: alterações do companion bloqueadas; histórico e
          auditoria continuam somente para leitura.
        </p>
      ) : null}
      {safeMode ? (
        <p role="alert">
          Modo seguro: alterações do companion bloqueadas; estado local
          permanece visível.
        </p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}

      <div className="message-actions">
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            void loadData().catch((loadError: unknown) =>
              setError(companionErrorMessage(loadError)),
            )
          }
        >
          Atualizar dispositivos, fila e auditoria
        </button>
        <button
          type="button"
          disabled={busy || blocked}
          onClick={() => void startPairing()}
        >
          Solicitar pareamento fixture
        </button>
      </div>

      <label>
        Dispositivo Android fixture
        <select
          value={selectedDeviceId}
          onChange={(event) => setSelectedDeviceId(event.target.value)}
          disabled={busy}
        >
          <option value="">Nenhum dispositivo</option>
          {devices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.deviceId} — {companionDeviceStatusLabels[device.status]}
            </option>
          ))}
        </select>
      </label>
      {selectedDevice ? (
        <>
          <p>
            Fingerprint: <code>{selectedDevice.fingerprint}</code>; chave v
            {selectedDevice.keyVersion}; fallback local ativo.
          </p>
          <div className="message-actions">
            {selectedDevice.status === "pairing_requested" ? (
              <button
                type="button"
                disabled={busy || blocked}
                onClick={() => void confirmPairing()}
              >
                Confirmar pareamento do Owner
              </button>
            ) : null}
            {selectedDevice.status === "paired" && !selectedSession ? (
              <button
                type="button"
                disabled={busy || blocked}
                onClick={() => void connectSelectedDevice()}
              >
                Conectar sessão local
              </button>
            ) : null}
            <button
              type="button"
              disabled={busy || blocked || selectedDevice.status !== "paired"}
              onClick={() => void rotateKey()}
            >
              Rotacionar chave
            </button>
            <button
              type="button"
              disabled={busy || blocked || selectedDevice.status === "revoked"}
              onClick={() => void revokeDevice()}
            >
              Revogar dispositivo
            </button>
          </div>
          <p>
            {selectedSession
              ? `Sessão ${selectedSession.id} autenticada; prova usa nonce e contador monotônico.`
              : "Nenhuma sessão conectada para este dispositivo."}
          </p>
        </>
      ) : (
        <p>Nenhum dispositivo fixture pareado.</p>
      )}

      <section>
        <h4>Fila offline e aprovação</h4>
        <p>
          O preview é metadata-only; aprovar não envia nem transporta conteúdo.
        </p>
        <div className="message-actions">
          <button
            type="button"
            disabled={busy || blocked || selectedSession === undefined}
            onClick={() => void previewQueue()}
          >
            Criar prévia de texto
          </button>
          <select
            value={selectedQueueId}
            onChange={(event) => setSelectedQueueId(event.target.value)}
            disabled={busy}
            aria-label="Item da fila do companion"
          >
            <option value="">Nenhum item</option>
            {queue.map((item) => (
              <option key={item.id} value={item.id}>
                {item.kind} — {companionQueueStatusLabels[item.status]}
              </option>
            ))}
          </select>
        </div>
        {selectedQueueItem ? (
          <>
            <p>
              {selectedQueueItem.summary}; aprovação obrigatória; metadata-only;
              bytes persistidos: não.
            </p>
            <div className="message-actions">
              <button
                type="button"
                disabled={
                  busy ||
                  blocked ||
                  selectedSession === undefined ||
                  selectedQueueItem.status !== "previewed"
                }
                onClick={() => void decideQueue(true)}
              >
                Aprovar item
              </button>
              <button
                type="button"
                disabled={
                  busy ||
                  blocked ||
                  selectedSession === undefined ||
                  !["previewed", "queued", "failed"].includes(
                    selectedQueueItem.status,
                  )
                }
                onClick={() => void actOnQueue("cancel_companion_queue")}
              >
                Cancelar item
              </button>
              <button
                type="button"
                disabled={
                  busy ||
                  blocked ||
                  selectedSession === undefined ||
                  !["cancelled", "failed"].includes(selectedQueueItem.status)
                }
                onClick={() => void actOnQueue("retry_companion_queue")}
              >
                Tentar novamente
              </button>
            </div>
          </>
        ) : null}
      </section>

      <section>
        <h4>Histórico e auditoria</h4>
        {history.length === 0 && audit.length === 0 ? (
          <p>Nenhum evento do companion registrado.</p>
        ) : (
          <ul>
            {audit.slice(0, 8).map((record) => (
              <li key={`audit-${record.id}`}>
                Auditoria: <strong>{record.event}</strong> — {record.summary}
              </li>
            ))}
            {history.slice(0, 8).map((record) => (
              <li key={`history-${record.id}`}>
                Histórico: <strong>{record.kind}</strong> — {record.summary}
              </li>
            ))}
          </ul>
        )}
        {rotations.length > 0 ? (
          <p>Última rotação: chave v{rotations[0]?.newKeyVersion} concluída.</p>
        ) : null}
        {revocations.length > 0 ? (
          <p>Última revogação: {revocations[0]?.reason}.</p>
        ) : null}
      </section>
    </section>
  );
}

const gatewayTransferStatusLabels: Record<string, string> = {
  previewed: "prévia aguardando aprovação",
  approved: "aprovada pelo Owner",
  revoked: "revogada",
};

const gatewaySessionStatusLabels: Record<string, string> = {
  connected: "conectada",
  disconnected: "desconectada",
  revoked: "revogada",
  expired: "expirada",
};

const gatewayRecoveryStatusLabels: Record<string, string> = {
  pending_approval: "aguardando aprovação",
  approved: "aprovada",
  revoked: "revogada",
};

const gatewayErrorLabels: Record<string, string> = {
  gateway_blocked_temporary: "Gateway bloqueado durante a conversa temporária.",
  gateway_blocked_safe_mode: "Gateway bloqueado pelo modo seguro.",
  gateway_blocked_suspended: "O agente suspenso não pode usar o gateway.",
  gateway_fixture_agent_invalid:
    "A agente fixture do gateway não é compatível.",
  gateway_agent_invalid: "A agente do gateway não está disponível localmente.",
  gateway_owner_required: "A aprovação do Owner local é necessária.",
  gateway_transfer_already_active:
    "Já existe uma transferência ativa para esta conta fixture.",
  gateway_transfer_revoked: "A transferência já foi revogada.",
  gateway_transfer_integrity_failed:
    "A integridade da transferência fixture não foi validada.",
  gateway_transfer_approval_required:
    "A aprovação explícita da transferência é necessária.",
  gateway_approval_required: "A aprovação explícita do Owner é necessária.",
  gateway_session_unavailable: "A sessão administrativa não está disponível.",
  gateway_authentication_failed: "A prova da sessão não foi autenticada.",
  gateway_replay_rejected:
    "A prova foi rejeitada por replay ou contador inválido.",
  gateway_protocol_incompatible: "A versão do protocolo não é compatível.",
  gateway_recovery_approval_required:
    "A recuperação ainda aguarda aprovação do Owner.",
  gateway_recovery_limit: "O limite de recuperações desta sessão foi atingido.",
};

function gatewayErrorMessage(error: unknown): string {
  const typed = parseCognitiveError(error);
  const code =
    typed?.code ??
    (typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "operation_unavailable");
  return (
    gatewayErrorLabels[code] ??
    "A operação do gateway local não está disponível."
  );
}

function gatewayIdempotencyKey(prefix: string): string {
  return `${prefix}-${crypto.randomUUID()}`;
}

function gatewayProof(
  session: GatewaySession,
  purpose: string,
): GatewaySessionProof {
  return {
    sessionId: session.id,
    transferId: session.transferId,
    clientId: session.clientId,
    sessionNonceMetadata: session.sessionNonceMetadata,
    authProofMetadata: session.authProofMetadata,
    appVersion: session.appVersion,
    protocolVersion: session.protocolVersion,
    messageNonceMetadata: `fixture:gateway-message/${purpose}-${crypto.randomUUID()}`,
    replayCounter: session.lastReplayCounter + 1,
  };
}

function parseGatewayPayload<T>(
  value: unknown,
  parser: (input: unknown) => T | null,
): T {
  const parsed = parser(value);
  if (parsed === null) throw new Error("gateway_payload_invalid");
  return parsed;
}

type GatewayTransportStartResult = {
  enabled: boolean;
  endpoint: string;
  pairingCode: string;
};
type GatewayTransportStatus = {
  enabled: boolean;
  endpoint: string | null;
  pairingAvailable: boolean;
};
const EMPTY_GATEWAY_TRANSPORT_STATUS: GatewayTransportStatus = {
  enabled: false,
  endpoint: null,
  pairingAvailable: false,
};
function parseGatewayTransportStatus(
  value: unknown,
): GatewayTransportStatus | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const candidate = value as Record<string, unknown>;
  return Object.keys(candidate).every((key) =>
    ["enabled", "endpoint", "pairingAvailable"].includes(key),
  ) &&
    typeof candidate.enabled === "boolean" &&
    (candidate.endpoint === null ||
      (typeof candidate.endpoint === "string" &&
        candidate.endpoint.length <= 128)) &&
    typeof candidate.pairingAvailable === "boolean"
    ? {
        enabled: candidate.enabled,
        endpoint: candidate.endpoint,
        pairingAvailable: candidate.pairingAvailable,
      }
    : null;
}
function isGatewayTransportStartResult(
  value: unknown,
): value is GatewayTransportStartResult {
  return (
    value !== null &&
    typeof value === "object" &&
    (value as Record<string, unknown>).enabled === true &&
    typeof (value as Record<string, unknown>).endpoint === "string" &&
    typeof (value as Record<string, unknown>).pairingCode === "string"
  );
}

export function GatewayControls({
  agentId,
  temporaryChat,
  safeMode,
}: {
  agentId: string;
  temporaryChat: boolean;
  safeMode: boolean;
}) {
  const [protocol, setProtocol] = useState<GatewayProtocolInfo | null>(null);
  const [accounts, setAccounts] = useState<GatewayAccount[]>([]);
  const [transfers, setTransfers] = useState<GatewayTransfer[]>([]);
  const [sessions, setSessions] = useState<GatewaySession[]>([]);
  const [recoveries, setRecoveries] = useState<GatewayRecovery[]>([]);
  const [audit, setAudit] = useState<GatewayAuditRecord[]>([]);
  const [revocations, setRevocations] = useState<GatewayRevocation[]>([]);
  const [transport, setTransport] = useState<GatewayTransportStatus>(
    EMPTY_GATEWAY_TRANSPORT_STATUS,
  );
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [selectedTransferId, setSelectedTransferId] = useState("");
  const [selectedSessionId, setSelectedSessionId] = useState("");
  const [selectedRecoveryId, setSelectedRecoveryId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const blocked = temporaryChat || safeMode;

  const loadData = useCallback(async () => {
    const [
      rawProtocol,
      rawAccounts,
      rawTransfers,
      rawSessions,
      rawRecoveries,
      rawAudit,
      rawRevocations,
      rawTransport,
    ] = await Promise.all([
      invoke<unknown>("get_gateway_protocol", { agentId }),
      invoke<unknown>("list_gateway_accounts", { agentId }),
      invoke<unknown>("list_gateway_transfers", { agentId }),
      invoke<unknown>("list_gateway_sessions", { agentId }),
      invoke<unknown>("list_gateway_recoveries", { agentId }),
      invoke<unknown>("list_gateway_audit", { agentId }),
      invoke<unknown>("list_gateway_revocations", { agentId }),
      invoke<unknown>("get_gateway_transport_status"),
    ]);
    const nextProtocol = parseGatewayPayload(
      rawProtocol,
      parseGatewayProtocolInfo,
    );
    const nextAccounts = parseGatewayPayload(rawAccounts, parseGatewayAccounts);
    const nextTransfers = parseGatewayPayload(
      rawTransfers,
      parseGatewayTransfers,
    );
    const nextSessions = parseGatewayPayload(rawSessions, parseGatewaySessions);
    const nextRecoveries = parseGatewayPayload(
      rawRecoveries,
      parseGatewayRecoveries,
    );
    const nextAudit = parseGatewayPayload(rawAudit, parseGatewayAudit);
    const nextRevocations = parseGatewayPayload(
      rawRevocations,
      parseGatewayRevocations,
    );
    const nextTransport =
      parseGatewayTransportStatus(rawTransport) ??
      EMPTY_GATEWAY_TRANSPORT_STATUS;
    setProtocol(nextProtocol);
    setAccounts(nextAccounts);
    setTransfers(nextTransfers);
    setSessions(nextSessions);
    setRecoveries(nextRecoveries);
    setAudit(nextAudit);
    setRevocations(nextRevocations);
    setTransport(nextTransport);
    setSelectedTransferId((current) =>
      nextTransfers.some((transfer) => transfer.id === current)
        ? current
        : (nextTransfers[0]?.id ?? ""),
    );
    setSelectedSessionId((current) =>
      nextSessions.some((session) => session.id === current)
        ? current
        : (nextSessions[0]?.id ?? ""),
    );
    setSelectedRecoveryId((current) =>
      nextRecoveries.some((recovery) => recovery.id === current)
        ? current
        : (nextRecoveries[0]?.id ?? ""),
    );
  }, [agentId]);

  useEffect(() => {
    void loadData().catch((loadError: unknown) =>
      setError(gatewayErrorMessage(loadError)),
    );
  }, [loadData]);

  const selectedTransfer = transfers.find(
    (transfer) => transfer.id === selectedTransferId,
  );
  const selectedSession = sessions.find(
    (session) => session.id === selectedSessionId,
  );
  const selectedRecovery = recoveries.find(
    (recovery) => recovery.id === selectedRecoveryId,
  );

  async function runMutation(operation: () => Promise<void>) {
    if (blocked || busy) return;
    setBusy(true);
    setError(null);
    try {
      await operation();
      await loadData();
    } catch (operationError: unknown) {
      setError(gatewayErrorMessage(operationError));
    } finally {
      setBusy(false);
    }
  }

  async function prepareTransfer() {
    await runMutation(async () => {
      const rawTransfer = await invoke<unknown>("prepare_gateway_transfer", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        destinationAccountMetadata: GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,
        integrityHash: GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,
        idempotencyKey: gatewayIdempotencyKey("transfer-prepare"),
        temporaryChat,
      });
      const transfer = parseGatewayPayload(rawTransfer, parseGatewayTransfer);
      setSelectedTransferId(transfer.id);
    });
  }

  async function approveTransfer() {
    if (!selectedTransfer) return;
    await runMutation(async () => {
      const rawTransfer = await invoke<unknown>("approve_gateway_transfer", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        transferId: selectedTransfer.id,
        approved: true,
        idempotencyKey: gatewayIdempotencyKey("transfer-approve"),
        temporaryChat,
      });
      parseGatewayPayload(rawTransfer, parseGatewayTransfer);
    });
  }

  async function connectSession() {
    if (!selectedTransfer || selectedTransfer.status !== "approved") return;
    await runMutation(async () => {
      const nextCounter =
        Math.max(
          0,
          ...sessions
            .filter((session) => session.clientId === GATEWAY_FIXTURE_CLIENT_ID)
            .map((session) => session.lastReplayCounter),
        ) + 1;
      const rawSession = await invoke<unknown>("connect_gateway_session", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        transferId: selectedTransfer.id,
        clientId: GATEWAY_FIXTURE_CLIENT_ID,
        appVersion: GATEWAY_FIXTURE_APP_VERSION,
        protocolVersion: GATEWAY_PROTOCOL_VERSION,
        authProofMetadata: GATEWAY_FIXTURE_AUTH_PROOF_METADATA,
        messageNonceMetadata: `fixture:gateway-message/connect-${crypto.randomUUID()}`,
        replayCounter: nextCounter,
        idempotencyKey: gatewayIdempotencyKey("session-connect"),
        temporaryChat,
      });
      const session = parseGatewayPayload(rawSession, parseGatewaySession);
      setSelectedSessionId(session.id);
    });
  }

  async function reconnectSession() {
    if (!selectedSession || selectedSession.status === "revoked") return;
    await runMutation(async () => {
      const rawSession = await invoke<unknown>("reconnect_gateway_session", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proof: gatewayProof(selectedSession, "session-reconnect"),
        idempotencyKey: gatewayIdempotencyKey("session-reconnect"),
        temporaryChat,
      });
      const session = parseGatewayPayload(rawSession, parseGatewaySession);
      setSelectedSessionId(session.id);
    });
  }

  async function requestRecovery() {
    if (!selectedSession || selectedSession.status !== "connected") return;
    await runMutation(async () => {
      const rawRecovery = await invoke<unknown>("request_gateway_recovery", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proof: gatewayProof(selectedSession, "recovery-request"),
        recoveryKind: "mobile_administrative",
        targetMetadata: GATEWAY_FIXTURE_RECOVERY_TARGET,
        idempotencyKey: gatewayIdempotencyKey("recovery-request"),
        temporaryChat,
      });
      const recovery = parseGatewayPayload(rawRecovery, parseGatewayRecovery);
      setSelectedRecoveryId(recovery.id);
    });
  }

  async function approveRecovery() {
    if (!selectedRecovery) return;
    const recoverySession = sessions.find(
      (session) => session.id === selectedRecovery.sessionId,
    );
    if (!recoverySession) return;
    await runMutation(async () => {
      const rawRecovery = await invoke<unknown>("approve_gateway_recovery", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        proof: gatewayProof(recoverySession, "recovery-approve"),
        recoveryId: selectedRecovery.id,
        approved: true,
        idempotencyKey: gatewayIdempotencyKey("recovery-approve"),
        temporaryChat,
      });
      parseGatewayPayload(rawRecovery, parseGatewayRecovery);
    });
  }

  async function revokeSession() {
    if (!selectedSession || selectedSession.status === "revoked") return;
    await runMutation(async () => {
      const rawRevocation = await invoke<unknown>("revoke_gateway_session", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        sessionId: selectedSession.id,
        reason: "Revogação solicitada pelo Owner no fixture local",
        idempotencyKey: gatewayIdempotencyKey("session-revoke"),
        temporaryChat,
      });
      parseGatewayPayload(rawRevocation, parseGatewayRevocation);
    });
  }

  async function revokeTransfer() {
    if (!selectedTransfer || selectedTransfer.status === "revoked") return;
    await runMutation(async () => {
      const rawRevocation = await invoke<unknown>("revoke_gateway_transfer", {
        agentId,
        ownerUserId: OWNER_USER_ID,
        transferId: selectedTransfer.id,
        reason: "Revogação solicitada pelo Owner no fixture local",
        idempotencyKey: gatewayIdempotencyKey("transfer-revoke"),
        temporaryChat,
      });
      parseGatewayPayload(rawRevocation, parseGatewayRevocation);
    });
  }

  async function startTransport() {
    if (blocked || busy) return;
    setBusy(true);
    setError(null);
    setPairingCode(null);
    try {
      const raw = await invoke<unknown>("start_gateway_transport", {
        agentId,
        ownerConfirmed: true,
        privateNetworkConfirmed: false,
        bindAddress: "127.0.0.1",
        port: 0,
        temporaryChat,
      });
      if (!isGatewayTransportStartResult(raw))
        throw new Error("gateway_response_invalid");
      setTransport({
        enabled: true,
        endpoint: raw.endpoint,
        pairingAvailable: true,
      });
      setPairingCode(raw.pairingCode);
    } catch (operationError: unknown) {
      setError(gatewayErrorMessage(operationError));
    } finally {
      setBusy(false);
    }
  }

  async function stopTransport() {
    if (busy) return;
    setBusy(true);
    setError(null);
    setPairingCode(null);
    try {
      await invoke("stop_gateway_transport");
      setTransport(EMPTY_GATEWAY_TRANSPORT_STATUS);
    } catch (operationError: unknown) {
      setError(gatewayErrorMessage(operationError));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="settings-card" aria-label="Gateway AIP local">
      <header>
        <h3>Gateway AIP local</h3>
        <p>
          Protocolo v{GATEWAY_PROTOCOL_VERSION}; fixture sintética; somente
          comandos Tauri locais.
        </p>
      </header>
      <ul>
        <li>
          TCP autenticado aip-gateway-v1, limitado ao local/privado e iniciado
          somente por ação explícita do Owner.
        </li>
        <li>
          Cloudflare é apenas configuração metadata; credenciais ausentes e sem
          credenciais, BielOS, relay ou túnel.
        </li>
        <li>
          Rust/SQLite mantém a autoridade sobre transferência, autenticação,
          recuperação, aprovação e revogação.
        </li>
      </ul>
      {temporaryChat ? (
        <p role="alert">
          Conversa temporária: mutações do gateway bloqueadas; estado e
          auditoria continuam somente para leitura.
        </p>
      ) : null}
      {safeMode ? (
        <p role="alert">
          Modo seguro: mutações do gateway bloqueadas; estado local permanece
          visível.
        </p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}

      <section aria-label="Ciclo de vida do gateway local">
        <h4>Listener local autenticado</h4>
        <p>
          Estado: <strong>{transport.enabled ? "ativo" : "parado"}</strong>;
          endpoint: <code>{transport.endpoint ?? "nenhum"}</code>; pairing
          disponível: {transport.pairingAvailable ? "sim" : "não"}.
        </p>
        <div className="message-actions">
          <button
            type="button"
            disabled={busy || blocked || transport.enabled}
            onClick={() => void startTransport()}
          >
            Iniciar gateway local
          </button>
          <button
            type="button"
            disabled={busy || !transport.enabled}
            onClick={() => void stopTransport()}
          >
            Parar gateway local
          </button>
        </div>
        {pairingCode ? (
          <p role="alert">
            <strong>Código de pairing transitório:</strong>{" "}
            <code>{pairingCode}</code>. Não persista nem compartilhe este
            código.
          </p>
        ) : null}
      </section>

      <div className="message-actions">
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            void loadData().catch((loadError: unknown) =>
              setError(gatewayErrorMessage(loadError)),
            )
          }
        >
          Atualizar estado e auditoria do gateway
        </button>
        <button
          type="button"
          disabled={busy || blocked}
          onClick={() => void prepareTransfer()}
        >
          Preparar transferência fixture
        </button>
      </div>

      <section>
        <h4>Protocolo, conta e transferência</h4>
        {protocol ? (
          <p>
            Transporte: <strong>{protocol.transport}</strong>; listener de rede:{" "}
            <strong>{protocol.networkListener ? "sim" : "não"}</strong>;
            fallback standalone:{" "}
            <strong>{protocol.standaloneFallback ? "sim" : "não"}</strong>.
          </p>
        ) : (
          <p>Protocolo ainda não carregado.</p>
        )}
        {protocol?.cloudflare ? (
          <p>
            Cloudflare: modo <strong>{protocol.cloudflare.mode}</strong>,
            credenciais <strong>{protocol.cloudflare.credentialState}</strong>,
            hostname metadata{" "}
            <code>{protocol.cloudflare.hostnameMetadata}</code>.
          </p>
        ) : null}
        {accounts.length === 0 ? (
          <p>Nenhuma conta metadata-only registrada.</p>
        ) : (
          <ul>
            {accounts.map((account) => (
              <li key={account.id}>
                Conta local <code>{account.localAccountId}</code>; externa
                metadata <code>{account.externalAccountIdMetadata}</code>;
                efeito externo: não.
              </li>
            ))}
          </ul>
        )}
        <label>
          Transferência fixture
          <select
            value={selectedTransferId}
            onChange={(event) => setSelectedTransferId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhuma transferência</option>
            {transfers.map((transfer) => (
              <option key={transfer.id} value={transfer.id}>
                {transfer.id} — {gatewayTransferStatusLabels[transfer.status]}
              </option>
            ))}
          </select>
        </label>
        {selectedTransfer ? (
          <>
            <p>
              Estado: {gatewayTransferStatusLabels[selectedTransfer.status]};
              aprovação obrigatória; metadata-only; efeito externo: não.
            </p>
            <div className="message-actions">
              <button
                type="button"
                disabled={
                  busy || blocked || selectedTransfer.status !== "previewed"
                }
                onClick={() => void approveTransfer()}
              >
                Aprovar transferência
              </button>
              <button
                type="button"
                disabled={
                  busy || blocked || selectedTransfer.status === "revoked"
                }
                onClick={() => void revokeTransfer()}
              >
                Revogar transferência
              </button>
              <button
                type="button"
                disabled={
                  busy || blocked || selectedTransfer.status !== "approved"
                }
                onClick={() => void connectSession()}
              >
                Conectar sessão local
              </button>
            </div>
          </>
        ) : null}
      </section>

      <section>
        <h4>Sessão administrativa e recuperação</h4>
        <label>
          Sessão gateway
          <select
            value={selectedSessionId}
            onChange={(event) => setSelectedSessionId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhuma sessão</option>
            {sessions.map((session) => (
              <option key={session.id} value={session.id}>
                {session.id} — {gatewaySessionStatusLabels[session.status]}
              </option>
            ))}
          </select>
        </label>
        {selectedSession ? (
          <>
            <p>
              Sessão {gatewaySessionStatusLabels[selectedSession.status]};
              autenticação{" "}
              {selectedSession.authenticated ? "válida" : "inválida"};
              transporte local:{" "}
              {selectedSession.localLoopbackOnly ? "sim" : "não"}.
            </p>
            <div className="message-actions">
              <button
                type="button"
                disabled={
                  busy || blocked || selectedSession.status === "revoked"
                }
                onClick={() => void reconnectSession()}
              >
                Reconectar sessão
              </button>
              <button
                type="button"
                disabled={
                  busy || blocked || selectedSession.status !== "connected"
                }
                onClick={() => void requestRecovery()}
              >
                Solicitar recuperação administrativa
              </button>
              <button
                type="button"
                disabled={
                  busy || blocked || selectedSession.status === "revoked"
                }
                onClick={() => void revokeSession()}
              >
                Revogar sessão
              </button>
            </div>
          </>
        ) : (
          <p>Nenhuma sessão administrativa fixture registrada.</p>
        )}
        <label>
          Recuperação
          <select
            value={selectedRecoveryId}
            onChange={(event) => setSelectedRecoveryId(event.target.value)}
            disabled={busy}
          >
            <option value="">Nenhuma recuperação</option>
            {recoveries.map((recovery) => (
              <option key={recovery.id} value={recovery.id}>
                {recovery.id} — {gatewayRecoveryStatusLabels[recovery.status]}
              </option>
            ))}
          </select>
        </label>
        {selectedRecovery ? (
          <>
            <p>
              Recuperação {gatewayRecoveryStatusLabels[selectedRecovery.status]}
              ; aprovação obrigatória; metadata-only; efeito externo: não.
            </p>
            <button
              type="button"
              disabled={
                busy ||
                blocked ||
                selectedRecovery.status !== "pending_approval"
              }
              onClick={() => void approveRecovery()}
            >
              Aprovar recuperação
            </button>
          </>
        ) : null}
      </section>

      <section>
        <h4>Auditoria e revogações</h4>
        {audit.length === 0 && revocations.length === 0 ? (
          <p>Nenhum evento do gateway local registrado.</p>
        ) : (
          <ul>
            {audit.slice(0, 8).map((record) => (
              <li key={`gateway-audit-${record.id}`}>
                Auditoria: <strong>{record.event}</strong> — {record.summary}
              </li>
            ))}
            {revocations.slice(0, 8).map((record) => (
              <li key={`gateway-revocation-${record.id}`}>
                Revogação: <strong>{record.targetKind}</strong> —{" "}
                {record.reason}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  );
}

function SettingsSurface({
  snapshot,
  changingMode,
  onToggleSafeMode,
}: {
  snapshot: AppSnapshot | null;
  changingMode: boolean;
  onToggleSafeMode: () => void;
}) {
  const [activeSection, setActiveSection] = useState("Geral");
  const sections = [
    "Geral",
    "Perfil do Owner",
    "Agentes",
    "Modelos",
    "Segurança",
    "Dados e backup",
    "Diagnóstico e Sobre",
  ];
  return (
    <section className="settings-surface">
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">A.I.P.</p>
          <h1>Configurações</h1>
          <span>Preferências locais e diagnóstico do aplicativo.</span>
        </div>
      </header>
      <div className="settings-layout">
        <nav className="settings-nav" aria-label="Seções de configurações">
          {sections.map((section) => (
            <button
              key={section}
              className={activeSection === section ? "active" : undefined}
              type="button"
              onClick={() => setActiveSection(section)}
            >
              {section}
            </button>
          ))}
        </nav>
        <div>
          {activeSection === "Geral" ? (
            <section className="settings-card">
              <h2>Geral</h2>
              <p>
                Este computador usa um Owner local implícito. Contas adicionais
                ainda não estão disponíveis.
              </p>
            </section>
          ) : null}
          {activeSection === "Perfil do Owner" ? (
            <section className="settings-card">
              <h2>Perfil do Owner</h2>
              <p>
                Administrador local do A.I.P. A edição do nome do Owner ainda
                não está disponível.
              </p>
            </section>
          ) : null}
          {activeSection === "Agentes" ? (
            <section className="settings-card">
              <h2>Agentes</h2>
              <p>Edite cada perfil pelo botão Perfil no espaço do agente.</p>
            </section>
          ) : null}
          {activeSection === "Modelos" ? (
            <section className="settings-card">
              <h2>Modelos</h2>
              <p>
                O modelo padrão é configurado por agente. Cada conversa pode ter
                uma substituição própria.
              </p>
            </section>
          ) : null}
          {activeSection === "Segurança" ? (
            <section className="settings-card">
              <h2>Segurança</h2>
              <p>O modo seguro desativa runtime, gerações e overlays.</p>
              <button
                className={
                  snapshot?.safeMode ? "mode-button active" : "mode-button"
                }
                type="button"
                disabled={!snapshot || changingMode}
                onClick={onToggleSafeMode}
              >
                {snapshot?.safeMode
                  ? "Sair do modo seguro"
                  : "Ativar modo seguro"}
              </button>
            </section>
          ) : null}
          {activeSection === "Dados e backup" ? (
            <section className="settings-card">
              <h2>Dados e backup</h2>
              <p>
                Exportação e backup automático ainda não estão disponíveis nesta
                versão.
              </p>
              <button type="button" disabled>
                Exportar dados (indisponível)
              </button>
            </section>
          ) : null}
          {activeSection === "Diagnóstico e Sobre" ? (
            <section className="settings-card">
              <h2>Diagnóstico e Sobre</h2>
              <dl>
                <dt>Versão</dt>
                <dd>{snapshot?.appVersion ?? "—"}</dd>
                <dt>Commit</dt>
                <dd>{snapshot?.buildSha ?? "—"}</dd>
                <dt>Build</dt>
                <dd>{snapshot?.buildTimestamp ?? "—"}</dd>
                <dt>Pacote</dt>
                <dd>{snapshot?.runtimePackagingMode ?? "—"}</dd>
                <dt>Runtime</dt>
                <dd>
                  {snapshot ? runtimeLabels[snapshot.runtime.state] : "—"}
                </dd>
                <dt>Detalhe</dt>
                <dd>{snapshot?.runtime.detailCode ?? "—"}</dd>
                <dt>Protocolo</dt>
                <dd>{snapshot?.runtime.protocolVersion ?? "—"}</dd>
                <dt>Banco local</dt>
                <dd>
                  {snapshot?.databaseReady ? "Disponível" : "Indisponível"}
                </dd>
                <dt>Migração</dt>
                <dd>{snapshot?.migrationVersion ?? "—"}</dd>
              </dl>
              <div className="message-actions">
                <button
                  type="button"
                  onClick={() =>
                    void navigator.clipboard?.writeText(
                      JSON.stringify({
                        version: snapshot?.appVersion,
                        build: snapshot?.buildSha,
                        runtime: snapshot?.runtime,
                        databaseReady: snapshot?.databaseReady,
                        migrationVersion: snapshot?.migrationVersion,
                        safeMode: snapshot?.safeMode,
                      }),
                    )
                  }
                >
                  Copiar diagnóstico
                </button>
                <button
                  type="button"
                  onClick={() => void invoke("retry_phase_one_runtime")}
                >
                  Reiniciar runtime
                </button>
              </div>
            </section>
          ) : null}
        </div>
      </div>
    </section>
  );
}

type LocalCapabilityState =
  | "Pronto"
  | "Não configurado"
  | "Indisponível"
  | "Bloqueado pelo modo"
  | "Erro";

type LocalCapabilityStatus = {
  state: LocalCapabilityState;
  detail: string;
};

type LocalCapabilityRead<T> = {
  value: T | null;
  outcome: "ready" | "unavailable" | "invalid";
};

type LocalCapabilityData = {
  ollama: LocalCapabilityRead<ProviderSnapshot>;
  voice: LocalCapabilityRead<VoiceProviderStatus>;
  devices: LocalCapabilityRead<VoiceDevice[]>;
  visualProvider: LocalCapabilityRead<ScreenVisionProviderStatus>;
  screenFixtures: LocalCapabilityRead<ScreenVisionFixture[]>;
  workspaceRoots: LocalCapabilityRead<WorkspaceRoot[]>;
  extensions: LocalCapabilityRead<ExtensionCatalogEntry[]>;
  companionDevices: LocalCapabilityRead<CompanionDevice[]>;
  companionTransport: LocalCapabilityRead<GatewayTransportStatus>;
  gatewayProtocol: LocalCapabilityRead<GatewayProtocolInfo>;
  gatewayTransport: LocalCapabilityRead<GatewayTransportStatus>;
};

function parseLocalVoiceDevices(value: unknown): VoiceDevice[] | null {
  if (!Array.isArray(value) || value.length > 64) return null;
  const devices = value.map(parseVoiceDevice);
  return devices.every((device): device is VoiceDevice => device !== null)
    ? devices
    : null;
}

async function readLocalCapability<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  parser: (value: unknown) => T | null,
): Promise<LocalCapabilityRead<T>> {
  try {
    const raw =
      args === undefined
        ? await invoke<unknown>(command)
        : await invoke<unknown>(command, args);
    const value = parser(raw);
    return value === null
      ? { value: null, outcome: "invalid" }
      : { value, outcome: "ready" };
  } catch {
    return { value: null, outcome: "unavailable" };
  }
}

function statusFromRead<T>(
  read: LocalCapabilityRead<T> | null,
  build: (value: T) => LocalCapabilityStatus,
): LocalCapabilityStatus {
  if (read === null) {
    return { state: "Indisponível", detail: "Verificando recurso local." };
  }
  if (read.outcome === "unavailable") {
    return { state: "Indisponível", detail: "O recurso local não respondeu." };
  }
  if (read.outcome === "invalid" || read.value === null) {
    return {
      state: "Erro",
      detail: "A resposta local não passou na validação.",
    };
  }
  return build(read.value);
}

function blockedLocalStatus(detail: string): LocalCapabilityStatus {
  return { state: "Bloqueado pelo modo", detail };
}

function runtimeLocalStatus(
  snapshot: AppSnapshot | null,
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode || snapshot?.runtime.state === "safe_mode") {
    return blockedLocalStatus("O modo seguro desativa o runtime de IA.");
  }
  if (snapshot === null) {
    return { state: "Indisponível", detail: "Aguardando o estado local." };
  }
  return snapshot.runtime.state === "ready"
    ? { state: "Pronto", detail: "Runtime local em execução." }
    : {
        state: "Indisponível",
        detail: "O histórico continua acessível sem o runtime.",
      };
}

function ollamaLocalStatus(
  read: LocalCapabilityRead<ProviderSnapshot> | null,
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode) {
    return blockedLocalStatus("O modo seguro bloqueia modelos e gerações.");
  }
  return statusFromRead(read, (provider) => {
    if (provider.state === "malformed") {
      return { state: "Erro", detail: "O provedor devolveu dados inválidos." };
    }
    if (provider.state === "available" && provider.models.length > 0) {
      return {
        state: "Pronto",
        detail: `${provider.models.length} modelo(s) local(is) encontrado(s).`,
      };
    }
    if (provider.state === "empty") {
      return {
        state: "Não configurado",
        detail: "Nenhum modelo local registrado.",
      };
    }
    return {
      state: "Indisponível",
      detail: "O runtime local ainda não disponibilizou modelos.",
    };
  });
}

function voiceLocalStatus(
  read: LocalCapabilityRead<VoiceProviderStatus> | null,
  kind: "recognition" | "synthesis",
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode) {
    return blockedLocalStatus("O modo seguro bloqueia operações de voz.");
  }
  return statusFromRead(read, (voice) => {
    const provider = voice[kind];
    if (provider.state === "ready") {
      return provider.synthetic
        ? {
            state: "Pronto",
            detail: "Fixture sintética pronta; não usa o dispositivo real.",
          }
        : { state: "Pronto", detail: "Provedor local pronto sob demanda." };
    }
    if (provider.state === "not_configured") {
      return {
        state: "Não configurado",
        detail: "Escolha um provedor local e suas referências.",
      };
    }
    return provider.state === "invalid"
      ? { state: "Erro", detail: "A referência do provedor não é válida." }
      : {
          state: "Indisponível",
          detail: "O provedor local não está disponível.",
        };
  });
}

function voiceDeviceLocalStatus(
  read: LocalCapabilityRead<VoiceDevice[]> | null,
  direction: VoiceDevice["direction"],
): LocalCapabilityStatus {
  return statusFromRead(read, (devices) => {
    const count = devices.filter(
      (device) => device.direction === direction,
    ).length;
    return count > 0
      ? {
          state: "Pronto",
          detail: `${count} dispositivo(s) local(is) detectado(s).`,
        }
      : {
          state: "Indisponível",
          detail: "Nenhum dispositivo compatível foi detectado.",
        };
  });
}

function visualProviderLocalStatus(
  read: LocalCapabilityRead<ScreenVisionProviderStatus> | null,
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode) {
    return blockedLocalStatus("O modo seguro bloqueia o uso visual.");
  }
  return statusFromRead(read, (provider) => {
    if (provider.state === "ready") {
      return {
        state: "Pronto",
        detail: "Provedor visual local configurado; captura exige confirmação.",
      };
    }
    if (provider.state === "not_configured") {
      return {
        state: "Não configurado",
        detail: "Configure um provedor visual local no ambiente.",
      };
    }
    return provider.state === "invalid"
      ? { state: "Erro", detail: "A configuração visual não é válida." }
      : {
          state: "Indisponível",
          detail: "O provedor visual não está disponível.",
        };
  });
}

function screenCaptureLocalStatus(
  read: LocalCapabilityRead<ScreenVisionFixture[]> | null,
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode) {
    return blockedLocalStatus("O modo seguro bloqueia captura e análise.");
  }
  return statusFromRead(read, (fixtures) => {
    const realFixtures = fixtures.filter(
      (fixture) => !fixture.synthetic,
    ).length;
    if (realFixtures > 0) {
      return {
        state: "Pronto",
        detail: `${realFixtures} monitor(es) local(is); privacidade e confirmação são obrigatórias.`,
      };
    }
    return fixtures.length > 0
      ? {
          state: "Não configurado",
          detail: "Somente fixture sintética disponível; falta captura real.",
        }
      : {
          state: "Indisponível",
          detail: "Nenhum monitor local foi detectado.",
        };
  });
}

function workspaceRootLocalStatus(
  read: LocalCapabilityRead<WorkspaceRoot[]> | null,
): LocalCapabilityStatus {
  return statusFromRead(read, (roots) => {
    const enabled = roots.filter((root) => root.enabled).length;
    return enabled > 0
      ? {
          state: "Pronto",
          detail: `${enabled} raiz(es) escolhida(s) pelo Owner está(ão) ativa(s).`,
        }
      : {
          state: "Não configurado",
          detail: "Adicione uma raiz local escolhida pelo Owner.",
        };
  });
}

function extensionLocalStatus(
  read: LocalCapabilityRead<ExtensionCatalogEntry[]> | null,
): LocalCapabilityStatus {
  return statusFromRead(read, (extensions) =>
    extensions.length > 0
      ? {
          state: "Pronto",
          detail: `${extensions.length} extensão(ões) no catálogo local; revisão e ativação são obrigatórias.`,
        }
      : {
          state: "Não configurado",
          detail: "Nenhuma extensão local foi registrada.",
        },
  );
}

function companionLocalStatus(
  devices: LocalCapabilityRead<CompanionDevice[]> | null,
  transport: LocalCapabilityRead<GatewayTransportStatus> | null,
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode) {
    return blockedLocalStatus(
      "O modo seguro bloqueia alterações do companion.",
    );
  }
  if (devices === null || transport === null) {
    return { state: "Indisponível", detail: "Verificando o companion local." };
  }
  if (devices.outcome === "invalid" || transport.outcome === "invalid") {
    return { state: "Erro", detail: "A resposta do companion não é válida." };
  }
  if (
    devices.outcome === "unavailable" ||
    transport.outcome === "unavailable"
  ) {
    return {
      state: "Indisponível",
      detail: "O companion local não respondeu.",
    };
  }
  const paired = devices.value?.some((device) => device.status === "paired");
  if (paired || transport.value?.enabled || transport.value?.pairingAvailable) {
    return {
      state: "Pronto",
      detail:
        "Fixture Android local disponível; pareamento e aprovação são obrigatórios.",
    };
  }
  return {
    state: "Não configurado",
    detail: "Nenhum dispositivo Android local está pareado.",
  };
}

function gatewayLocalStatus(
  protocol: LocalCapabilityRead<GatewayProtocolInfo> | null,
  transport: LocalCapabilityRead<GatewayTransportStatus> | null,
  safeMode: boolean,
): LocalCapabilityStatus {
  if (safeMode) {
    return blockedLocalStatus("O modo seguro bloqueia alterações do gateway.");
  }
  if (protocol === null || transport === null) {
    return { state: "Indisponível", detail: "Verificando o gateway local." };
  }
  if (protocol.outcome === "invalid" || transport.outcome === "invalid") {
    return { state: "Erro", detail: "A resposta do gateway não é válida." };
  }
  if (
    protocol.outcome === "unavailable" ||
    transport.outcome === "unavailable"
  ) {
    return { state: "Indisponível", detail: "O gateway local não respondeu." };
  }
  return transport.value?.enabled
    ? {
        state: "Pronto",
        detail:
          "Gateway local em loopback; efeitos externos permanecem desativados.",
      }
    : {
        state: "Não configurado",
        detail:
          "O protocolo local está disponível; iniciar exige aprovação do Owner.",
      };
}

export function LocalCapabilityStatusCenter({
  agentId,
  snapshot,
  safeMode,
  temporaryChat,
}: {
  agentId: string;
  snapshot: AppSnapshot | null;
  safeMode: boolean;
  temporaryChat: boolean;
}) {
  const [data, setData] = useState<LocalCapabilityData | null>(null);

  useEffect(() => {
    let mounted = true;
    async function load() {
      const [
        ollama,
        voice,
        devices,
        visualProvider,
        screenFixtures,
        workspaceRoots,
        extensions,
        companionDevices,
        companionTransport,
        gatewayProtocol,
        gatewayTransport,
      ] = await Promise.all([
        readLocalCapability(
          "get_ollama_status",
          undefined,
          parseProviderSnapshot,
        ),
        readLocalCapability(
          "get_voice_provider_status",
          { agentId },
          parseVoiceProviderStatus,
        ),
        readLocalCapability(
          "list_voice_devices",
          undefined,
          parseLocalVoiceDevices,
        ),
        readLocalCapability(
          "get_screen_vision_provider_status",
          undefined,
          parseScreenVisionProviderStatus,
        ),
        readLocalCapability(
          "list_screen_vision_fixtures",
          undefined,
          parseScreenVisionFixtures,
        ),
        readLocalCapability(
          "list_workspace_roots",
          undefined,
          parseWorkspaceRoots,
        ),
        readLocalCapability(
          "list_extension_catalog",
          { agentId },
          parseExtensionCatalog,
        ),
        readLocalCapability(
          "list_companion_devices",
          { agentId },
          parseCompanionDevices,
        ),
        readLocalCapability(
          "get_companion_transport_status",
          undefined,
          parseGatewayTransportStatus,
        ),
        readLocalCapability(
          "get_gateway_protocol",
          { agentId },
          parseGatewayProtocolInfo,
        ),
        readLocalCapability(
          "get_gateway_transport_status",
          undefined,
          parseGatewayTransportStatus,
        ),
      ]);
      if (mounted) {
        setData({
          ollama,
          voice,
          devices,
          visualProvider,
          screenFixtures,
          workspaceRoots,
          extensions,
          companionDevices,
          companionTransport,
          gatewayProtocol,
          gatewayTransport,
        });
      }
    }
    void load();
    return () => {
      mounted = false;
    };
  }, [agentId]);

  const runtimeStatus = runtimeLocalStatus(snapshot, safeMode);
  const ollamaStatus = ollamaLocalStatus(data?.ollama ?? null, safeMode);
  const cards = [
    {
      key: "runtime",
      label: "Runtime",
      href: "#local-capability-runtime",
      status: runtimeStatus,
    },
    {
      key: "ollama",
      label: "Ollama",
      href: "#local-capability-runtime",
      status: ollamaStatus,
    },
    {
      key: "stt",
      label: "STT",
      href: "#local-capability-voice",
      status: voiceLocalStatus(data?.voice ?? null, "recognition", safeMode),
    },
    {
      key: "tts",
      label: "TTS",
      href: "#local-capability-voice",
      status: voiceLocalStatus(data?.voice ?? null, "synthesis", safeMode),
    },
    {
      key: "microphone",
      label: "Microfone",
      href: "#local-capability-voice",
      status: voiceDeviceLocalStatus(data?.devices ?? null, "input"),
    },
    {
      key: "audio-output",
      label: "Saída de áudio",
      href: "#local-capability-voice",
      status: voiceDeviceLocalStatus(data?.devices ?? null, "output"),
    },
    {
      key: "visual-provider",
      label: "Provedor visual",
      href: "#local-capability-screen-vision",
      status: visualProviderLocalStatus(data?.visualProvider ?? null, safeMode),
    },
    {
      key: "screen-capture",
      label: "Captura de tela",
      href: "#local-capability-screen-vision",
      status: screenCaptureLocalStatus(data?.screenFixtures ?? null, safeMode),
    },
    {
      key: "workspace-root",
      label: "Raiz de workspace",
      href: "#local-capability-tools",
      status: workspaceRootLocalStatus(data?.workspaceRoots ?? null),
    },
    {
      key: "extensions",
      label: "Extensões",
      href: "#local-capability-extensions",
      status: extensionLocalStatus(data?.extensions ?? null),
    },
    {
      key: "companion",
      label: "Companion Android",
      href: "#local-capability-companion",
      status: companionLocalStatus(
        data?.companionDevices ?? null,
        data?.companionTransport ?? null,
        safeMode,
      ),
    },
    {
      key: "gateway",
      label: "Gateway",
      href: "#local-capability-gateway",
      status: gatewayLocalStatus(
        data?.gatewayProtocol ?? null,
        data?.gatewayTransport ?? null,
        safeMode,
      ),
    },
  ];

  return (
    <section
      className="local-status-center"
      id="local-status-center"
      aria-labelledby="local-status-heading"
    >
      <header>
        <p className="eyebrow">Diagnóstico local</p>
        <h2 id="local-status-heading">Centro de status local</h2>
        <span>
          Leitura segura de hardware, providers e integrações; nenhuma
          referência de executável é exibida.
        </span>
      </header>
      {temporaryChat ? (
        <p className="local-mode-note" role="status">
          Conversa temporária: leituras continuam visíveis, mas mutações e
          gravações ficam bloqueadas.
        </p>
      ) : null}
      {safeMode ? (
        <p className="local-mode-note" role="status">
          Modo seguro: runtime e alterações de capabilities ficam bloqueados.
        </p>
      ) : null}
      <p className="local-status-legend">
        Estados possíveis: Pronto, Não configurado, Indisponível, Bloqueado pelo
        modo e Erro.
      </p>
      <div className="local-status-grid">
        {cards.map((card) => (
          <a
            className="local-status-card"
            data-state={card.status.state}
            href={card.href}
            aria-controls={card.href.slice(1)}
            key={card.key}
            onClick={(event) => {
              const panel = document.getElementById(card.href.slice(1));
              if (!(panel instanceof HTMLDetailsElement)) return;
              event.preventDefault();
              panel.open = true;
              panel.scrollIntoView?.({ behavior: "smooth", block: "start" });
              panel.querySelector("summary")?.focus();
            }}
          >
            <span>{card.label}</span>
            <strong>{card.status.state}</strong>
            <small>{card.status.detail}</small>
          </a>
        ))}
      </div>
      <details className="local-capability-panel" id="local-capability-runtime">
        <summary>Runtime e Ollama</summary>
        <div>
          <p>
            <strong>Runtime:</strong> {runtimeStatus.state} —{" "}
            {runtimeStatus.detail}
          </p>
          <p>
            <strong>Ollama:</strong> {ollamaStatus.state} —{" "}
            {ollamaStatus.detail}
          </p>
          <p>
            Configuração: mantenha o Ollama local ativo e use “Atualizar” na
            conversa para reler os modelos. Sem runtime, o histórico e as
            leituras continuam acessíveis.
          </p>
        </div>
      </details>
    </section>
  );
}

export function LocalCapabilitiesSurface({
  agentId,
  snapshot,
  safeMode,
  temporaryChat,
}: {
  agentId: string;
  snapshot: AppSnapshot | null;
  safeMode: boolean;
  temporaryChat: boolean;
}) {
  return (
    <section className="local-capabilities-surface">
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">Capacidades do Owner local</p>
          <h1>Recursos locais</h1>
          <span>
            Fixtures sintéticas servem para demonstração; hardware e providers
            reais dependem do Windows e da configuração local.
          </span>
        </div>
      </header>
      <LocalCapabilityStatusCenter
        agentId={agentId}
        snapshot={snapshot}
        safeMode={safeMode}
        temporaryChat={temporaryChat}
      />
      <p className="local-capability-note">
        Pré-requisitos: runtime para geração, referências locais para STT/TTS,
        dispositivos Windows para áudio, provedor visual para análise, raiz do
        Owner para ferramentas locais e aprovação explícita para extensões,
        companion e gateway. O histórico e as leituras permanecem acessíveis se
        o runtime estiver indisponível.
      </p>
      <div className="local-capability-panels">
        <details className="local-capability-panel" id="local-capability-voice">
          <summary>Voz local</summary>
          <VoiceControls
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
        <details className="local-capability-panel" id="local-capability-tools">
          <summary>Ferramentas supervisionadas e workspace</summary>
          <ToolControls
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
        <details
          className="local-capability-panel"
          id="local-capability-extensions"
        >
          <summary>Extensões locais</summary>
          <ExtensionControls
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
        <details
          className="local-capability-panel"
          id="local-capability-screen-vision"
        >
          <summary>Visão de tela sintética</summary>
          <ScreenVisionControls
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
        <details
          className="local-capability-panel"
          id="local-capability-companion"
        >
          <summary>Companion Android local</summary>
          <CompanionControls
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
        <details
          className="local-capability-panel"
          id="local-capability-gateway"
        >
          <summary>Gateway AIP local</summary>
          <GatewayControls
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
        <details
          className="local-capability-panel"
          id="local-capability-cognitive"
        >
          <summary>Valores cognitivos</summary>
          <CognitivePanelGate
            agentId={agentId}
            temporaryChat={temporaryChat}
            safeMode={safeMode}
          />
        </details>
      </div>
    </section>
  );
}

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [activeAgentId, setActiveAgentId] = useState<string | null>(null);
  const [changingMode, setChangingMode] = useState(false);
  const [editingAgentId, setEditingAgentId] = useState<string | null>(null);
  const [conversationRevision, setConversationRevision] = useState(0);
  const [conversationNavigationRevision, setConversationNavigationRevision] =
    useState(0);
  const [conversationListRevision, setConversationListRevision] = useState(0);
  const [conversationDraftAgentId, setConversationDraftAgentId] = useState<
    string | null
  >(null);
  const [conversationDraftRevision, setConversationDraftRevision] = useState(0);
  const [temporaryChat, setTemporaryChat] = useState(false);
  const [workspace, setWorkspace] = useState<
    "chat" | "memories" | "state" | "appearance" | "resources" | "settings"
  >("chat");

  const loadSnapshot = useCallback(async () => {
    const next = await invoke<AppSnapshot>("get_app_snapshot");
    setSnapshot(next);
    setActiveAgentId((current) => current ?? next.agents[0]?.id ?? null);
  }, []);

  useEffect(() => {
    void loadSnapshot();
    const timer = window.setInterval(() => void loadSnapshot(), 1500);
    return () => window.clearInterval(timer);
  }, [loadSnapshot]);

  useEffect(() => {
    const registration = createListenerRegistration();
    void listen<OpenAgentConversationsPayload>(
      OPEN_AGENT_CONVERSATIONS_EVENT,
      (event) => {
        const { agentId } = event.payload;
        setConversationDraftAgentId(null);
        setActiveAgentId(agentId);
        setEditingAgentId(null);
        setWorkspace("chat");
        setConversationNavigationRevision((value) => value + 1);
      },
    ).then(registration.register);
    return registration.dispose;
  }, []);

  async function toggleSafeMode() {
    if (!snapshot || changingMode) return;
    setChangingMode(true);
    try {
      setSnapshot(
        await invoke<AppSnapshot>("set_safe_mode", {
          enabled: !snapshot.safeMode,
        }),
      );
    } finally {
      setChangingMode(false);
    }
  }

  async function openWorkspace(
    next:
      "chat" | "memories" | "state" | "appearance" | "resources" | "settings",
  ) {
    setConversationDraftAgentId(null);
    await leaveTemporaryChat();
    setEditingAgentId(null);
    setWorkspace(next);
  }

  async function openProfile(agentId: string) {
    setConversationDraftAgentId(null);
    await leaveTemporaryChat();
    setWorkspace("chat");
    setEditingAgentId(agentId);
  }

  async function leaveTemporaryChat() {
    if (temporaryChat && activeAgentId !== null) {
      await invoke("close_temporary_phase_one_chat", {
        agentId: activeAgentId,
      });
    }
    setTemporaryChat(false);
  }

  async function toggleTemporaryChat() {
    setConversationDraftAgentId(null);
    if (temporaryChat) {
      await leaveTemporaryChat();
      return;
    }
    setEditingAgentId(null);
    setWorkspace("chat");
    setTemporaryChat(true);
  }

  async function selectAgent(agentId: string) {
    setConversationDraftAgentId(null);
    await leaveTemporaryChat();
    setActiveAgentId(agentId);
    setEditingAgentId(null);
    setWorkspace("chat");
  }

  function openConversationDraft() {
    if (activeAgentId === null) return;
    setConversationDraftAgentId(activeAgentId);
    setConversationDraftRevision((value) => value + 1);
    setEditingAgentId(null);
    setWorkspace("chat");
    void leaveTemporaryChat();
  }

  return (
    <div className="app-shell conversation-layout">
      <aside className="sidebar" aria-label="Navegação principal">
        <button
          className="brand-mark"
          type="button"
          aria-label="Abrir configurações"
          onClick={() => void openWorkspace("settings")}
        >
          <img className="brand-logo" src="/icon.ico" alt="" />
          <div>
            <strong>A.I.P.</strong>
            <small>Conversa local</small>
          </div>
        </button>
        <p className="sidebar-label">Conversas</p>
        <div className="agent-tabs">
          {snapshot?.agents.map((agent) => (
            <AgentButton
              key={agent.id}
              agent={agent}
              active={agent.id === activeAgentId}
              onSelect={() => void selectAgent(agent.id)}
            />
          ))}
        </div>
        {activeAgentId ? (
          <ConversationList
            key={`${activeAgentId}-${conversationListRevision}`}
            agentId={activeAgentId}
            onNewDraft={openConversationDraft}
            onSelectExisting={() => setConversationDraftAgentId(null)}
            changed={() => {
              void leaveTemporaryChat().then(() => {
                setConversationDraftAgentId(null);
                setConversationRevision((value) => value + 1);
                setEditingAgentId(null);
                setWorkspace("chat");
              });
            }}
          />
        ) : null}
        {activeAgentId ? (
          <nav className="sidebar-secondary" aria-label="Áreas do agente">
            <p className="sidebar-label">Este agente</p>
            {(
              [
                ["memories", "Memórias"],
                ["state", "Estado"],
                ["appearance", "Aparência"],
              ] as const
            ).map(([key, label]) => (
              <button
                key={key}
                className={workspace === key ? "active" : undefined}
                type="button"
                aria-current={workspace === key ? "page" : undefined}
                onClick={() => void openWorkspace(key)}
              >
                {label}
              </button>
            ))}
            <button
              type="button"
              onClick={() => void openProfile(activeAgentId)}
            >
              Perfil de{" "}
              {snapshot?.agents.find((agent) => agent.id === activeAgentId)
                ?.name ?? "agente"}
            </button>
            <p className="sidebar-label">Aplicativo</p>
            <button
              className={workspace === "resources" ? "active" : undefined}
              type="button"
              aria-current={workspace === "resources" ? "page" : undefined}
              onClick={() => void openWorkspace("resources")}
            >
              Recursos locais
            </button>
            <button
              className={workspace === "settings" ? "active" : undefined}
              type="button"
              aria-current={workspace === "settings" ? "page" : undefined}
              onClick={() => void openWorkspace("settings")}
            >
              Configurações
            </button>
          </nav>
        ) : null}
        <div className="sidebar-footer">
          <span className="local-dot" aria-hidden="true" />
          {snapshot
            ? runtimeLabels[snapshot.runtime.state]
            : "Verificando runtime"}
        </div>
      </aside>
      <main className="conversation-main">
        {snapshot?.runtime.state === "unavailable" ||
        snapshot?.runtime.state === "crashed" ? (
          <div className="runtime-banner" role="status">
            <span>
              {runtimeLabels[snapshot.runtime.state]}. O histórico continua
              disponível.
            </span>
            <button
              type="button"
              onClick={() => void invoke("retry_phase_one_runtime")}
            >
              Reiniciar runtime
            </button>
          </div>
        ) : null}
        {snapshot?.onboardingRequired && snapshot.agents.length === 2 ? (
          <OnboardingForm
            agents={snapshot.agents}
            done={() => {
              setEditingAgentId(null);
              void loadSnapshot();
            }}
          />
        ) : editingAgentId !== null &&
          snapshot?.agents.find((agent) => agent.id === editingAgentId) ? (
          <ProfileForm
            agent={snapshot.agents.find(
              (agent) => agent.id === editingAgentId,
            )!}
            done={() => {
              setEditingAgentId(null);
              void loadSnapshot();
            }}
          />
        ) : activeAgentId === null ? (
          <section className="conversation-empty">Carregando agentes…</section>
        ) : workspace === "settings" ? (
          <section className="workspace-panel">
            <SettingsSurface
              snapshot={snapshot}
              changingMode={changingMode}
              onToggleSafeMode={() => void toggleSafeMode()}
            />
          </section>
        ) : workspace === "resources" ? (
          <section className="workspace-panel">
            <LocalCapabilitiesSurface
              agentId={activeAgentId}
              snapshot={snapshot}
              safeMode={snapshot?.safeMode ?? true}
              temporaryChat={temporaryChat}
            />
          </section>
        ) : workspace === "memories" ? (
          <section className="workspace-panel">
            <MemoryWorkspace agentId={activeAgentId} />
          </section>
        ) : workspace === "state" ? (
          <section className="workspace-panel">
            <section className="state-workspace">
              <header className="workspace-heading">
                <div>
                  <p className="eyebrow">Ritmo do agente</p>
                  <h2>Estado</h2>
                  <span>
                    Escolha como este agente pode aparecer e responder.
                  </span>
                </div>
              </header>
              <AgentStateControls agentId={activeAgentId} />
            </section>
          </section>
        ) : workspace === "appearance" ? (
          <section className="workspace-panel">
            <section className="appearance-workspace">
              <header className="workspace-heading">
                <div>
                  <p className="eyebrow">Visual 64 × 64</p>
                  <h2>Aparência</h2>
                  <span>Edite em etapas: ferramenta, camada e grade.</span>
                </div>
              </header>
              <PixelDocumentEditor agentId={activeAgentId} />
            </section>
          </section>
        ) : conversationDraftAgentId === activeAgentId && !temporaryChat ? (
          <ConversationDraftSurface
            key={`${activeAgentId}-${conversationDraftRevision}`}
            agentId={activeAgentId}
            onCreated={() => setConversationListRevision((value) => value + 1)}
            onPersisted={() => {
              setConversationDraftAgentId(null);
              setConversationRevision((value) => value + 1);
              setConversationListRevision((value) => value + 1);
              setWorkspace("chat");
            }}
          />
        ) : (
          <ConversationSurface
            key={`${activeAgentId}-${conversationRevision}-${temporaryChat}`}
            agentId={activeAgentId}
            temporary={temporaryChat}
            refreshRevision={conversationNavigationRevision}
            onToggleTemporary={() => void toggleTemporaryChat()}
          />
        )}
      </main>
    </div>
  );
}

export default App;
