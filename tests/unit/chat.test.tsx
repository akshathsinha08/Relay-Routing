import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { api, Channel } = vi.hoisted(() => {
  class Channel<T> {
    onmessage: ((event: T) => void) | null = null;
  }
  return {
    Channel,
    api: {
      continueGeneration: vi.fn(),
      sendMessage: vi.fn(),
      listMessages: vi.fn(),
      listConversations: vi.fn(),
      attachmentPreview: vi.fn(),
    },
  };
});

vi.mock("@tauri-apps/api/core", () => ({ Channel }));
vi.mock("@/lib/api", () => ({
  api,
  errorMessage: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

import { MessageList } from "@/components/MessageList";
import { useChatStore } from "@/stores/chat";
import { useProvidersStore } from "@/stores/providers";
import { useUiStore } from "@/stores/ui";
import type { Account, Conversation, MessageView } from "@/types";

const conversation: Conversation = {
  id: "c1",
  projectId: null,
  projectName: null,
  title: "Continuity",
  pinned: false,
  archived: false,
  providerId: "ollama",
  accountId: "ollama-default",
  model: "gemma4:12b",
  summary: null,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  lastMessageAt: null,
  lastMessage: null,
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

const interruptedView: MessageView = {
  message: {
    id: "a1",
    conversationId: "c1",
    seq: 2,
    role: "assistant",
    content: "first",
    reasoning: null,
    status: "error",
    error: "network reset",
    interrupted: true,
    interruptionReason: "network reset",
    segments: [
      {
        id: "s1",
        messageId: "a1",
        generationId: "g1",
        attemptId: "at1",
        seq: 1,
        content: "first",
        reasoning: null,
        status: "interrupted",
        providerId: "ollama",
        accountId: "ollama-default",
        model: "gemma4:12b",
        inputTokens: 2,
        outputTokens: 1,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      },
    ],
    providerId: "ollama",
    accountId: "ollama-default",
    model: "gemma4:12b",
    inputTokens: 2,
    outputTokens: 1,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  attachments: [],
};

function resetState() {
  useUiStore.setState({ toasts: [], view: { name: "chat" } });
  useProvidersStore.setState({ accounts: [account] });
  useChatStore.setState({
    conversations: [conversation],
    activeId: "c1",
    messages: [interruptedView],
    attachments: [],
    loading: false,
    searchQuery: "",
    streaming: {
      active: false,
      assistantId: null,
      content: "",
      reasoning: "",
      error: null,
      attempt: null,
      routing: null,
      statusLine: null,
    },
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetState();
});

describe("chat continuity", () => {
  it("marks the message streaming and applies attempt, delta, and usage events", async () => {
    let release!: () => void;
    api.continueGeneration.mockImplementation(
      async (
        _conversationId: string,
        _messageId: string,
        channel: InstanceType<typeof Channel>,
      ) => {
        channel.onmessage?.({
          type: "attemptStarted",
          generationId: "g1",
          attemptNo: 1,
          providerId: "ollama",
          accountId: "ollama-default",
          model: "gemma4:12b",
        });
        channel.onmessage?.({
          type: "segmentStarted",
          messageId: "a1",
          segmentId: "s2",
          seq: 2,
        });
        channel.onmessage?.({ type: "delta", content: " second" });
        channel.onmessage?.({
          type: "usage",
          inputTokens: 4,
          outputTokens: 2,
        });
        await new Promise<void>((resolve) => {
          release = resolve;
        });
      },
    );
    api.listMessages.mockImplementation(async () => useChatStore.getState().messages);
    api.listConversations.mockResolvedValue([]);

    const pending = useChatStore.getState().continueResponse("a1");
    expect(useChatStore.getState().streaming.active).toBe(true);

    const state = useChatStore.getState();
    expect(state.streaming.attempt).toBe(1);
    expect(state.streaming.routing).toEqual({
      providerId: "ollama",
      accountId: "ollama-default",
      model: "gemma4:12b",
    });
    const message = state.messages[0]!.message;
    expect(message.content).toBe("first second");
    expect(message.status).toBe("streaming");
    expect(message.inputTokens).toBe(4);
    expect(message.outputTokens).toBe(2);
    expect(message.interrupted).toBe(false);

    release();
    await expect(pending).resolves.toBe(true);
    expect(useChatStore.getState().streaming.active).toBe(false);
  });

  it("renders an interrupted response with Continue and segment attribution", () => {
    render(<MessageList />);

    expect(screen.getByText("Interrupted")).toBeInTheDocument();
    expect(screen.queryByText("Error")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Continue/ })).toBeEnabled();
    expect(screen.getByText("gemma4:12b · Local · partial")).toBeInTheDocument();
  });
});
