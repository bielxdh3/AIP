// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GatewayControls } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const protocol = {
  schemaVersion: 1,
  protocolVersion: 1,
  minProtocolVersion: 1,
  transport: "local_loopback_fixture",
  networkListener: false,
  cloudflare: {
    provider: "cloudflare_tunnel_access",
    mode: "metadata_only",
    tunnelIdMetadata: "fixture:tunnel/aip-gateway",
    hostnameMetadata: "example.invalid",
    accessAudienceMetadata: "fixture:access/aip-owner",
    credentialState: "absent",
    networkListener: false,
  },
  standaloneFallback: true,
};

const transfer = {
  id: "transfer-fixture-1",
  accountId: "gateway-account-owner",
  sourceAgentId: "agt_luma_provisional",
  ownerUserId: "usr_owner_local",
  destinationAccountMetadata: "fixture:external-account/bielos-owner",
  integrityHash: "sha256:fixture/girlfriend-agent-v1",
  status: "previewed",
  authorizationStatus: "pending_owner_approval",
  approvalRequired: true,
  metadataOnly: true,
  externalEffectPerformed: false,
  standaloneFallback: true,
  createdAt: 1,
  approvedAt: null,
  updatedAt: 1,
};

describe("GatewayControls", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    invoke.mockReset();
  });

  async function renderGateway(
    temporaryChat = false,
    safeMode = false,
  ): Promise<HTMLDivElement> {
    const nextContainer = document.createElement("div");
    container = nextContainer;
    document.body.append(nextContainer);
    root = createRoot(nextContainer);
    await act(async () =>
      root?.render(
        <GatewayControls
          agentId="agt_luma_provisional"
          temporaryChat={temporaryChat}
          safeMode={safeMode}
        />,
      ),
    );
    return nextContainer;
  }

  it("loads protocol metadata and read-only gateway records locally", async () => {
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "get_gateway_protocol" ? protocol : []),
    );

    await renderGateway();

    expect(container?.textContent).toContain("Gateway AIP local");
    expect(container?.textContent).toContain(
      "Cloudflare é apenas configuração metadata",
    );
    expect(container?.textContent).toContain(
      "TCP autenticado aip-gateway-v1",
    );
    expect(invoke).toHaveBeenCalledWith("get_gateway_protocol", {
      agentId: "agt_luma_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_gateway_accounts", {
      agentId: "agt_luma_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_gateway_transfers", {
      agentId: "agt_luma_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_gateway_sessions", {
      agentId: "agt_luma_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_gateway_recoveries", {
      agentId: "agt_luma_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_gateway_audit", {
      agentId: "agt_luma_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_gateway_revocations", {
      agentId: "agt_luma_provisional",
    });
  });

  it("delegates transfer preparation to Rust and fails closed in blocked modes", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "get_gateway_protocol") return Promise.resolve(protocol);
      if (command === "prepare_gateway_transfer")
        return Promise.resolve(transfer);
      return Promise.resolve([]);
    });

    await renderGateway();
    const prepareButton = [
      ...(container?.querySelectorAll("button") ?? []),
    ].find((button) => button.textContent === "Preparar transferência fixture");
    await act(async () => prepareButton?.click());

    expect(invoke).toHaveBeenCalledWith(
      "prepare_gateway_transfer",
      expect.objectContaining({
        agentId: "agt_luma_provisional",
        ownerUserId: "usr_owner_local",
        destinationAccountMetadata: "fixture:external-account/bielos-owner",
        integrityHash: "sha256:fixture/girlfriend-agent-v1",
        temporaryChat: false,
      }),
    );

    invoke.mockClear();
    await act(async () => root?.unmount());
    container?.remove();
    container = undefined;
    root = undefined;
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "get_gateway_protocol" ? protocol : []),
    );
    const blockedContainer = await renderGateway(true, true);

    const blockedPrepareButton = [
      ...blockedContainer.querySelectorAll("button"),
    ].find((button) => button.textContent === "Preparar transferência fixture");
    expect(blockedPrepareButton?.disabled).toBe(true);
    expect(invoke).not.toHaveBeenCalledWith(
      "prepare_gateway_transfer",
      expect.anything(),
    );
    expect(blockedContainer.textContent).toContain(
      "Conversa temporária: mutações do gateway bloqueadas",
    );
    expect(blockedContainer.textContent).toContain(
      "Modo seguro: mutações do gateway bloqueadas",
    );
  });

  it("loads status and starts/stops the listener with transient pairing", async () => {
    const status = { enabled: false, endpoint: null, pairingAvailable: false };
    invoke.mockImplementation((command: string) => {
      if (command === "get_gateway_protocol") return Promise.resolve(protocol);
      if (command === "get_gateway_transport_status") return Promise.resolve(status);
      if (command === "start_gateway_transport") return Promise.resolve({ enabled: true, endpoint: "127.0.0.1:43123", pairingCode: "transient-fixture-code" });
      return Promise.resolve([]);
    });
    await renderGateway();
    expect(invoke).toHaveBeenCalledWith("get_gateway_transport_status");
    const start = [...(container?.querySelectorAll("button") ?? [])].find((button) => button.textContent === "Iniciar gateway local");
    await act(async () => start?.click());
    expect(invoke).toHaveBeenCalledWith("start_gateway_transport", { agentId: "agt_luma_provisional", ownerConfirmed: true, privateNetworkConfirmed: false, bindAddress: "127.0.0.1", port: 0, temporaryChat: false });
    expect(container?.textContent).toContain("transient-fixture-code");
    const stop = [...(container?.querySelectorAll("button") ?? [])].find((button) => button.textContent === "Parar gateway local");
    await act(async () => stop?.click());
    expect(invoke).toHaveBeenCalledWith("stop_gateway_transport");
    expect(container?.textContent).not.toContain("transient-fixture-code");
  });

  it("blocks listener mutation in temporary and safe modes", async () => {
    invoke.mockImplementation((command: string) => Promise.resolve(command === "get_gateway_protocol" ? protocol : []));
    await renderGateway(true, true);
    const start = [...(container?.querySelectorAll("button") ?? [])].find((button) => button.textContent === "Iniciar gateway local");
    expect(start?.disabled).toBe(true);
    expect(invoke).not.toHaveBeenCalledWith("start_gateway_transport", expect.anything());
  });
});
