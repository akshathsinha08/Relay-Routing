import { beforeEach, describe, expect, it, vi } from "vitest";

const { api } = vi.hoisted(() => ({
  api: {
    listProjects: vi.fn(),
    getProject: vi.fn(),
    listProjectAttachments: vi.fn(),
    listProjectArtifacts: vi.fn(),
    createProject: vi.fn(),
    updateProject: vi.fn(),
    deleteProject: vi.fn(),
    importAttachments: vi.fn(),
    deleteAttachment: vi.fn(),
    listProviders: vi.fn(),
    listAccounts: vi.fn(),
    listModels: vi.fn(),
    getSettings: vi.fn(),
  },
}));

vi.mock("@/lib/api", () => ({
  api,
  errorMessage: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

import { useProjectsStore } from "@/stores/projects";
import { useProvidersStore } from "@/stores/providers";
import { useUiStore } from "@/stores/ui";
import type { Account, ModelInfo, Project, Provider } from "@/types";

const project: Project = {
  id: "p1",
  name: "Release notes",
  description: "Notes",
  instructions: "Be concise",
  archived: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  conversationCount: 0,
};

const provider: Provider = {
  id: "ollama",
  kind: "ollama",
  name: "Ollama",
  baseUrl: "http://127.0.0.1:11434",
  enabled: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const account: Account = {
  id: "ollama-default",
  providerId: "ollama",
  label: "Local",
  authKind: "none",
  status: "ok",
  statusMessage: null,
  metadataJson: "{}",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  hasSecret: false,
};

const model: ModelInfo = {
  id: "qwen3.5:4b",
  name: "qwen3.5:4b",
  sizeBytes: 2_600_000_000,
  family: "qwen",
  parameterSize: "4B",
  quantization: "Q4_K_M",
  capabilities: ["completion", "tools"],
  modifiedAt: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  useUiStore.setState({ toasts: [], view: { name: "chat" } });
  useProjectsStore.setState({
    projects: [],
    active: null,
    attachments: [],
    artifacts: [],
    loading: false,
  });
  useProvidersStore.setState({
    providers: [],
    accounts: [],
    models: {},
    health: {},
    loading: false,
  });
});

describe("projects store", () => {
  it("creates a project then loads and opens it", async () => {
    api.createProject.mockResolvedValue(project);
    api.listProjects.mockResolvedValue([project]);
    api.getProject.mockResolvedValue(project);
    api.listProjectAttachments.mockResolvedValue([]);
    api.listProjectArtifacts.mockResolvedValue([]);

    const created = await useProjectsStore.getState().create("Release notes", "Notes");
    expect(created?.id).toBe("p1");
    expect(useProjectsStore.getState().projects).toHaveLength(1);

    await useProjectsStore.getState().open("p1");
    expect(useProjectsStore.getState().active?.name).toBe("Release notes");
    expect(api.listProjectAttachments).toHaveBeenCalledWith("p1");
  });

  it("surfaces a toast instead of throwing when creation fails", async () => {
    api.createProject.mockRejectedValue(new Error("disk full"));

    const created = await useProjectsStore.getState().create("Broken");
    expect(created).toBeNull();
    expect(useUiStore.getState().toasts.at(0)?.text).toBe("disk full");
  });

  it("removes a project and clears it when active", async () => {
    useProjectsStore.setState({ projects: [project], active: project });
    api.deleteProject.mockResolvedValue(undefined);

    await useProjectsStore.getState().remove("p1");
    expect(useProjectsStore.getState().projects).toHaveLength(0);
    expect(useProjectsStore.getState().active).toBeNull();
  });
});

describe("providers store", () => {
  it("selects the enabled Ollama account and first completion model", async () => {
    api.listModels.mockResolvedValue([model]);
    useProvidersStore.setState({ providers: [provider], accounts: [account] });

    const selection = await useProvidersStore.getState().defaultSelection();
    expect(selection).toEqual({
      providerId: "ollama",
      accountId: "ollama-default",
      model: "qwen3.5:4b",
    });
    expect(api.listModels).toHaveBeenCalledWith("ollama", "ollama-default");
    expect(useProvidersStore.getState().cachedModels("ollama", "ollama-default")).toEqual([
      model,
    ]);
  });

  it("does not reload models that are already cached", async () => {
    useProvidersStore.setState({
      providers: [provider],
      accounts: [account],
      models: { "ollama:ollama-default": [model] },
    });

    await useProvidersStore.getState().ensureModels("ollama", "ollama-default");
    expect(api.listModels).not.toHaveBeenCalled();
  });

  it("returns null when no provider is enabled", async () => {
    useProvidersStore.setState({
      providers: [{ ...provider, enabled: false }],
      accounts: [account],
    });
    expect(await useProvidersStore.getState().defaultSelection()).toBeNull();
  });
});

describe("ui store", () => {
  it("auto-dismisses toasts after five seconds", () => {
    vi.useFakeTimers();
    useUiStore.getState().toast("Saved", "success");
    expect(useUiStore.getState().toasts).toHaveLength(1);
    vi.advanceTimersByTime(5000);
    expect(useUiStore.getState().toasts).toHaveLength(0);
    vi.useRealTimers();
  });
});
