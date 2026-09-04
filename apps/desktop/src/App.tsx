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
import { ThemeControls } from "./theme";
import { AipSelect, FilePicker } from "./shared-controls";
import {
  MODEL_POLICY_MODES,
  type ModelPolicyMode,
  routingPolicyPayload,
  useModelPreferences,
} from "./model-preferences";
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

export type DesktopWorkspace =
  "chat" | "memories" | "state" | "appearance" | "resources" | "settings";

export function SidebarNavigation({
  agents,
  activeAgentId,
  workspace,
  onSelectAgent,
  onWorkspace,
  onProfile,
}: {
  agents: ProvisionalAgent[];
  activeAgentId: string | null;
  workspace: DesktopWorkspace;
  onSelectAgent: (agentId: string) => void;
  onWorkspace: (workspace: DesktopWorkspace) => void;
  onProfile: (agentId: string) => void;
}) {
  const activeAgent = agents.find((agent) => agent.id === activeAgentId);
  return (
    <div className="sidebar-navigation">
      <details className="sidebar-section sidebar-agents" open>
        <summary>
          <span>Agentes</span>
          <small>{agents.length}</small>
        </summary>
        <div className="agent-tabs">
          {agents.map((agent) => (
            <AgentButton
              key={agent.id}
              agent={agent}
              active={agent.id === activeAgentId}
              onSelect={() => onSelectAgent(agent.id)}
            />
          ))}
        </div>
      </details>
      {activeAgentId ? (
        <details className="sidebar-section sidebar-secondary" open>
          <summary>
            <span>Navegação</span>
            <small>{activeAgent?.name ?? "agente"}</small>
          </summary>
          <nav aria-label="Áreas do agente">
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
                onClick={() => onWorkspace(key)}
              >
                {label}
              </button>
            ))}
            <button type="button" onClick={() => onProfile(activeAgentId)}>
              Perfil de {activeAgent?.name ?? "agente"}
            </button>
            <p className="sidebar-label">Aplicativo</p>
            <button
              className={workspace === "resources" ? "active" : undefined}
              type="button"
              aria-current={workspace === "resources" ? "page" : undefined}
              onClick={() => onWorkspace("resources")}
            >
              Recursos locais
            </button>
            <button
              className={workspace === "settings" ? "active" : undefined}
              type="button"
              aria-current={workspace === "settings" ? "page" : undefined}
              onClick={() => onWorkspace("settings")}
            >
              Configurações
            </button>
          </nav>
        </details>
      ) : null}
    </div>
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

type ModelPickerPosition = {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
  placement: "above" | "below";
  ready: boolean;
};

const MODEL_PICKER_VIEWPORT_PADDING = 8;
const MODEL_PICKER_MENU_GAP = 4;
const MODEL_PICKER_MIN_HEIGHT = 40;

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

function modelOption(
  model: OllamaModel,
  unavailable = false,
): ModelPickerOption {
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
    unavailable,
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
  const popoverRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const listboxId = useId();
  const [modelPreferences] = useModelPreferences();
  const [position, setPosition] = useState<ModelPickerPosition>({
    top: 0,
    left: 0,
    width: 240,
    maxHeight: 320,
    placement: "above",
    ready: false,
  });

  const visibleModels = models.filter(
    (model) => !modelPreferences.hiddenModelRefs.includes(model.ref),
  );
  const hiddenSelectedModel =
    value === null
      ? undefined
      : models.find(
          (model) =>
            model.ref === value &&
            modelPreferences.hiddenModelRefs.includes(model.ref),
        );
  const pickerModels =
    hiddenSelectedModel === undefined
      ? visibleModels
      : [...visibleModels, hiddenSelectedModel];

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
    ...pickerModels.map((model) =>
      modelOption(model, modelPreferences.hiddenModelRefs.includes(model.ref)),
    ),
    ...(value !== null && !pickerModels.some((model) => model.ref === value)
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

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const popover = popoverRef.current;
    if (trigger === null || popover === null) return;
    const rect = trigger.getBoundingClientRect();
    const menuHeight =
      popover.getBoundingClientRect().height ||
      Math.min(filteredOptions.length * 48 + 48, 360);
    const viewportWidth = Math.max(window.innerWidth, 1);
    const viewportHeight = Math.max(window.innerHeight, 1);
    const spaceBelow = Math.max(
      0,
      viewportHeight - rect.bottom - MODEL_PICKER_VIEWPORT_PADDING,
    );
    const spaceAbove = Math.max(0, rect.top - MODEL_PICKER_VIEWPORT_PADDING);
    const placement =
      menuHeight > spaceBelow && spaceAbove > spaceBelow ? "above" : "below";
    const availableHeight = placement === "above" ? spaceAbove : spaceBelow;
    const maxHeight = Math.max(
      MODEL_PICKER_MIN_HEIGHT,
      Math.min(menuHeight, availableHeight),
    );
    const maxWidth = Math.max(
      160,
      viewportWidth - MODEL_PICKER_VIEWPORT_PADDING * 2,
    );
    const width = Math.min(Math.max(rect.width, 220), maxWidth);
    const left = Math.min(
      Math.max(MODEL_PICKER_VIEWPORT_PADDING, rect.left),
      Math.max(
        MODEL_PICKER_VIEWPORT_PADDING,
        viewportWidth - width - MODEL_PICKER_VIEWPORT_PADDING,
      ),
    );
    const requestedTop =
      placement === "above"
        ? rect.top - Math.min(menuHeight, maxHeight) - MODEL_PICKER_MENU_GAP
        : rect.bottom + MODEL_PICKER_MENU_GAP;
    const top = Math.min(
      Math.max(MODEL_PICKER_VIEWPORT_PADDING, requestedTop),
      Math.max(
        MODEL_PICKER_VIEWPORT_PADDING,
        viewportHeight - maxHeight - MODEL_PICKER_VIEWPORT_PADDING,
      ),
    );
    setPosition({ top, left, width, maxHeight, placement, ready: true });
  }, [filteredOptions.length]);

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
        !rootRef.current.contains(event.target) &&
        !popoverRef.current?.contains(event.target)
      ) {
        setOpen(false);
        setQuery("");
      }
    }
    document.addEventListener("pointerdown", closeFromOutside);
    return () => document.removeEventListener("pointerdown", closeFromOutside);
  }, [open]);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
    const frame = window.requestAnimationFrame(updatePosition);
    const onViewportChange = () => updatePosition();
    window.addEventListener("resize", onViewportChange);
    window.addEventListener("scroll", onViewportChange, true);
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", onViewportChange);
      window.removeEventListener("scroll", onViewportChange, true);
    };
  }, [open, updatePosition]);

  useEffect(() => {
    if (!open) setPosition((current) => ({ ...current, ready: false }));
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
            <small className="readable-helper">{triggerDetail}</small>
          </span>
          <span aria-hidden="true">⌄</span>
        </button>
        {open ? (
          <div
            ref={popoverRef}
            className="model-picker-popover"
            data-placement={position.placement}
            style={{
              top: position.top,
              left: position.left,
              right: "auto",
              width: position.width,
              maxHeight: position.maxHeight,
              visibility: position.ready ? "visible" : "hidden",
            }}
          >
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
                    <small className="readable-helper">{option.detail}</small>
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
      <small className="model-picker-status readable-helper">
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
  onActiveConversationChange,
}: {
  agentId: string;
  temporary: boolean;
  onToggleTemporary?: () => void;
  refreshRevision?: number;
  onActiveConversationChange?: (conversationId: string) => void;
}) {
  const { phase, error, load } = usePhaseOne(agentId, temporary);
  const [modelPreferences] = useModelPreferences();
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
  const phaseConversationId = phase?.conversation.id;

  useEffect(() => {
    if (refreshRevision > 0) void load();
  }, [load, refreshRevision]);

  useEffect(() => {
    if (!temporary && phaseConversationId !== undefined) {
      onActiveConversationChange?.(phaseConversationId);
    }
  }, [onActiveConversationChange, phaseConversationId, temporary]);

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
  const providerUnavailable = phase.provider.state !== "available";
  const modelsAvailable = phase.provider.models.length > 0;
  const canSend =
    canSendConversationMessage(currentPhase) && !providerUnavailable;
  const canDraft = canDraftConversationMessage(currentPhase);
  const routingPolicy = routingPolicyPayload(modelPreferences);

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
          ? { agentId: currentPhase.agent.id, content, policy: routingPolicy }
          : {
              agentId: currentPhase.agent.id,
              conversationId: currentPhase.conversation.id,
              content,
              policy: routingPolicy,
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
          <h1>{phase.agent.name}</h1>
          <span className="conversation-title">{phase.conversation.title}</span>
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
        <div className="composer-status" aria-live="polite">
          <span className={`provider-state ${phase.provider.state}`}>
            {providerStatusCopy(phase)}
          </span>
          {providerRecovery ? (
            <p className="provider-recovery" role="status">
              {providerRecovery}
            </p>
          ) : null}
          {temporary ? (
            <p className="temporary-disclosure readable-helper" role="status">
              Temporária ativa: mensagens e contexto ficam apenas na memória e
              são apagados ao encerrar.
            </p>
          ) : null}
        </div>
        {request !== null ? (
          <div className="queue-banner">
            <span
              className={
                request.active && !request.cancellationRequested
                  ? "generation-status shiny-text"
                  : "generation-status"
              }
            >
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
            {onToggleTemporary ? (
              <button
                className={
                  temporary ? "temporary-control active" : "temporary-control"
                }
                type="button"
                aria-label={
                  temporary
                    ? "Encerrar conversa temporária"
                    : "Iniciar conversa temporária"
                }
                aria-pressed={temporary}
                title={
                  temporary
                    ? "Encerrar conversa temporária"
                    : "Iniciar conversa temporária"
                }
                onClick={onToggleTemporary}
              >
                <span aria-hidden="true">◌</span>
              </button>
            ) : null}
            <div className="conversation-model-selector">
              <ModelPicker
                label="Modelo desta conversa"
                ariaLabel="Modelo desta conversa"
                models={phase.provider.models}
                value={phase.modelOverrideRef}
                providerState={phase.provider.state}
                disabled={!modelsAvailable}
                defaultOption={{
                  label: "Modelo: Automático",
                  detail: phase.defaultModelRef
                    ? `Equilibrado · ${phase.defaultModelRef}`
                    : "Equilibrado · seleção automática",
                }}
                statusText={
                  phase.modelOverrideRef
                    ? `Conversa · ${phase.selectedModelRef ?? "indisponível"}`
                    : phase.selectedModelRef
                      ? `${phase.effectiveModelSource === "temporary_override" ? "Temporária" : "Agente"} · ${phase.selectedModelRef}`
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
            <details className="model-advanced-controls">
              <summary>Mais opções</summary>
              <div>
                <button type="button" onClick={() => void refreshModels()}>
                  Atualizar modelos
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
  const [modelPreferences] = useModelPreferences();
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
  const providerRecovery = providerRecoveryCopy(currentPhase);
  const providerUnavailable = currentPhase.provider.state !== "available";
  const canSend =
    canSendConversationMessage(currentPhase) && !providerUnavailable;
  const canDraft = canDraftConversationMessage(currentPhase);
  const routingPolicy = routingPolicyPayload(modelPreferences);

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
          policy: routingPolicy,
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
          <h1>{currentPhase.agent.name}</h1>
          <span className="conversation-title">
            Ainda não foi salvo no histórico
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
        <div className="composer-status" aria-live="polite">
          <span className={`provider-state ${currentPhase.provider.state}`}>
            {providerStatusCopy(currentPhase)}
          </span>
          {providerRecovery ? (
            <p className="provider-recovery" role="status">
              {providerRecovery}
            </p>
          ) : null}
        </div>
        {request !== null ? (
          <div className="queue-banner">
            <span
              className={
                request.active && !request.cancellationRequested
                  ? "generation-status shiny-text"
                  : "generation-status"
              }
            >
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
  const options = profileCanonicalOptions[field].some(
    (option) => option.value === value,
  )
    ? profileCanonicalOptions[field]
    : [{ value, primary: