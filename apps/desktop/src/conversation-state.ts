import type {
  ConversationMessage,
  PhaseOneEvent,
  PhaseOneState,
  QueueEntry,
} from "@aip/contracts";

export type ConversationViewState = {
  phase: PhaseOneState;
  lastSequenceByRequest: Record<string, number>;
};

export function conversationOverrideArguments(
  agentId: string,
  conversationId: string,
  modelRef: string,
) {
  return { agentId, conversationId, modelRef: modelRef || null };
}

const terminalStatuses = new Set(["complete", "failed", "cancelled"]);

export type RequestTerminalState =
  "completed" | "cancelled" | "failed_provider" | "failed_other";

export function requestTerminalState(
  eventType: PhaseOneEvent["eventType"],
  errorCode: string | null = null,
): RequestTerminalState | null {
  switch (eventType) {
    case "generation.complete":
      return "completed";
    case "generation.cancelled":
      return "cancelled";
    case "generation.failed":
      return errorCode === "model_unavailable" ||
        errorCode?.startsWith("provider_")
        ? "failed_provider"
        : "failed_other";
    default:
      return null;
  }
}

export function createConversationViewState(
  phase: PhaseOneState,
): ConversationViewState {
  return { phase, lastSequenceByRequest: {} };
}

export function applyPhaseOneEvent(
  state: ConversationViewState,
  event: PhaseOneEvent,
): ConversationViewState {
  if (
    event.protocolVersion !== 1 ||
    event.eventType === "state.changed" ||
    event.agentId !== state.phase.agent.id ||
    event.conversationId !== state.phase.conversation.id ||
    event.requestId === null ||
    event.assistantMessageId === null
  ) {
    return state;
  }

  const messageIndex = state.phase.messages.findIndex(
    (message) => message.id === event.assistantMessageId,
  );
  if (messageIndex < 0) return state;
  const current = state.phase.messages[messageIndex];
  if (current === undefined || terminalStatuses.has(current.status))
    return state;

  const queueEntry = state.phase.queue.find(
    (entry) =>
      entry.requestId === event.requestId &&
      entry.assistantMessageId === event.assistantMessageId,
  );
  if (queueEntry === undefined && event.eventType === "generation.chunk") {
    return state;
  }

  if (event.eventType === "generation.chunk") {
    if (event.sequence === null || event.content === null) return state;
    const previous = state.lastSequenceByRequest[event.requestId] ?? 0;
    if (event.sequence !== previous + 1) return state;
    return updateMessage(
      state,
      messageIndex,
      {
        ...current,
        content: current.content + event.content,
        status: "streaming",
      },
      {
        ...state.lastSequenceByRequest,
        [event.requestId]: event.sequence,
      },
    );
  }

  const terminal = requestTerminalState(event.eventType, event.errorCode);
  if (terminal === null) {
    if (event.eventType !== "generation.started") return state;
    return updateMessage(state, messageIndex, {
      ...current,
      status: "streaming",
    });
  }
  const lastSequence = state.lastSequenceByRequest[event.requestId] ?? 0;
  const cancellationSequenceCanAdvance =
    event.eventType === "generation.cancelled" &&
    event.sequence !== null &&
    event.sequence >= lastSequence;
  if (
    event.sequence !== null &&
    !cancellationSequenceCanAdvance &&
    event.sequence !== lastSequence
  ) {
    return state;
  }
  const nextSequences = { ...state.lastSequenceByRequest };
  delete nextSequences[event.requestId];
  return updateMessage(
    {
      ...state,
      phase: {
        ...state.phase,
        queue: state.phase.queue.filter(
          (entry) =>
            entry.requestId !== event.requestId &&
            entry.assistantMessageId !== event.assistantMessageId,
        ),
      },
      lastSequenceByRequest: nextSequences,
    },
    messageIndex,
    {
      ...current,
      status:
        terminal === "completed"
          ? "complete"
          : terminal === "cancelled"
            ? "cancelled"
            : "failed",
      errorCode: event.errorCode,
    },
  );
}

function updateMessage(
  state: ConversationViewState,
  index: number,
  message: ConversationMessage,
  sequences = state.lastSequenceByRequest,
): ConversationViewState {
  const messages = [...state.phase.messages];
  messages[index] = message;
  return {
    phase: { ...state.phase, messages },
    lastSequenceByRequest: sequences,
  };
}

export function requestForAgent(
  queue: QueueEntry[],
  agentId: string,
): QueueEntry | null {
  return queue.find((entry) => entry.agentId === agentId) ?? null;
}

export function canSendConversationMessage(state: PhaseOneState): boolean {
  return state.canSend && requestForAgent(state.queue, state.agent.id) === null;
}

export function canDraftConversationMessage(state: PhaseOneState): boolean {
  return state.canSend || requestForAgent(state.queue, state.agent.id) !== null;
}

export function canRequestCancellation(
  request: QueueEntry | null,
  locallyPendingRequestId: string | null,
): boolean {
  return (
    request !== null &&
    !request.cancellationRequested &&
    locallyPendingRequestId !== request.requestId
  );
}

export function messageStatusCopy(message: ConversationMessage): string {
  if (message.status === "failed") {
    return message.content
      ? "Resposta interrompida"
      : "Não foi possível gerar a resposta";
  }
  const labels: Record<ConversationMessage["status"], string> = {
    pending: "Aguardando processamento…",
    streaming: "Gerando resposta…",
    complete: "Concluída",
    failed: "Não foi possível gerar a resposta",
    cancelled: "Resposta cancelada",
  };
  return labels[message.status];
}

export function messageFailureCopy(errorCode: string | null): string {
  if (
    errorCode === "runtime_unavailable" ||
    errorCode === "provider_unavailable" ||
    errorCode === "orchestration_unavailable"
  ) {
    return "Servidor de IA indisponível.";
  }
  if (errorCode === "runtime_interrupted") {
    return "Conexão com o servidor perdida.";
  }
  if (
    errorCode === "provider_model_unavailable" ||
    errorCode === "model_unavailable"
  ) {
    return "O modelo selecionado não está disponível";
  }
  if (errorCode === "provider_timeout") {
    return "O modelo demorou demais para responder";
  }
  if (
    errorCode === "provider_stream_closed" ||
    errorCode === "provider_interrupted"
  ) {
    return "A resposta foi interrompida. O texto recebido foi preservado";
  }
  if (errorCode?.startsWith("provider_")) {
    return "O Ollama não conseguiu concluir a resposta. Tente novamente";
  }
  if (errorCode?.startsWith("protocol_")) {
    return "A comunicação com o runtime falhou. Tente reiniciar o runtime";
  }
  if (errorCode === "persistence_failed") {
    return "A resposta não pôde ser salva corretamente";
  }
  if (errorCode?.startsWith("runtime_")) {
    return "O runtime local está indisponível. Reinicie-o e tente novamente";
  }
  return "Não foi possível gerar a resposta";
}

export function providerStatusCopy(state: PhaseOneState): string {
  if (!state.selectedModelAvailable && state.selectedModelRef !== null) {
    return "Modelo selecionado indisponível";
  }
  switch (state.provider.state) {
    case "checking":
      return "Verificando Ollama…";
    case "available":
      return "Ollama disponível";
    case "empty":
      return "Nenhum modelo instalado";
    case "malformed":
      return "Resposta inválida do Ollama";
    case "timeout":
      return "Ollama não respondeu a tempo";
    case "unavailable":
      return "Ollama indisponível";
  }
}

export function providerRecoveryCopy(state: PhaseOneState): string | null {
  switch (state.provider.state) {
    case "checking":
      return "Procurando os modelos instalados no Ollama…";
    case "unavailable":
      return "O Ollama não está ativo. Abra o Ollama e tente atualizar os modelos.";
    case "empty":
      return "O Ollama está ativo, mas ainda não há um modelo instalado.";
    case "timeout":
      return "O Ollama demorou para responder. Tente atualizar novamente.";
    case "malformed":
      return "O Ollama respondeu em um formato que o A.I.P. não reconheceu.";
    case "available":
      return null;
  }
}

export function blockedSendCopy(code: string | null): string | null {
  switch (code) {
    case null:
      return null;
    case "safe_mode_active":
      return "Saia do modo seguro para conversar.";
    case "agent_suspended":
      return "Retome este agente para conversar.";
    case "runtime_unavailable":
    case "provider_unavailable":
    case "orchestration_unavailable":
      return "Servidor de IA indisponível.";
    case "runtime_interrupted":
      return "Conexão com o servidor perdida.";
    case "provider_checking":
      return "Processando agora.";
    case "provider_empty":
    case "model_unavailable":
      return "Modelo não instalado.";
    case "model_not_selected":
      return "Selecione um modelo local.";
    case "selected_model_unavailable":
      return "Modelo selecionado indisponível.";
    case "no_candidate":
      return "Nenhum dispositivo disponível para este modelo.";
    case "queue_full":
      return "Servidor ocupado. Sua solicitação está na fila.";
    default:
      return "Não foi possível iniciar a conversa.";
  }
}

export function compactPreview(content: string): string {
  const lines = content.trim().split(/\r?\n/);
  return lines.length <= 3
    ? lines.join("\n")
    : `${lines.slice(0, 3).join("\n")}…`;
}

export type BubblePresentation = {
  preview: string;
  fullText: string;
  request: QueueEntry | null;
  canReply: boolean;
};

export function bubblePresentation(state: PhaseOneState): BubblePresentation {
  const request = requestForAgent(state.queue, state.agent.id);
  const latestAgent = [...state.messages]
    .reverse()
    .find((message) => message.author === "agent");
  const fullText = latestAgent?.content ?? "";
  const preview = request
    ? request.cancellationRequested
      ? "Cancelando resposta…"
      : request.active
        ? "Gerando resposta…"
        : "Aguardando processamento…"
    : fullText
      ? compactPreview(fullText)
      : providerStatusCopy(state);
  return { preview, fullText, request, canReply: state.canSend };
}
