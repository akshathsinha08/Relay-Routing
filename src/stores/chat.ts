import { Channel } from "@tauri-apps/api/core";
import { create } from "zustand";

import { api, errorMessage } from "@/lib/api";
import type {
  Attachment,
  Conversation,
  Message,
  MessageView,
  StreamEvent,
} from "@/types";

import { useProvidersStore } from "./providers";
import { useUiStore } from "./ui";

interface StreamingState {
  active: boolean;
  assistantId: string | null;
  content: string;
  reasoning: string;
  error: string | null;
  attempt: number | null;
  routing: {
    providerId: string;
    accountId: string;
    model: string;
  } | null;
  statusLine: string | null;
}

const idleStreaming: StreamingState = {
  active: false,
  assistantId: null,
  content: "",
  reasoning: "",
  error: null,
  attempt: null,
  routing: null,
  statusLine: null,
};

interface ChatState {
  conversations: Conversation[];
  activeId: string | null;
  messages: MessageView[];
  attachments: Attachment[];
  streaming: StreamingState;
  loading: boolean;
  searchQuery: string;
  setSearchQuery: (query: string) => Promise<void>;
  loadConversations: (opts?: {
    projectId?: string;
    query?: string;
    includeArchived?: boolean;
  }) => Promise<void>;
  openConversation: (id: string) => Promise<void>;
  newConversation: (projectId?: string) => Promise<Conversation | null>;
  send: (content: string) => Promise<boolean>;
  continueResponse: (messageId: string) => Promise<boolean>;
  cancel: () => Promise<void>;
  attachFiles: (paths: string[]) => Promise<void>;
  removeAttachment: (id: string) => Promise<void>;
  renameConversation: (id: string, title: string) => Promise<void>;
  togglePin: (id: string) => Promise<void>;
  archiveConversation: (id: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  moveConversation: (id: string, projectId: string | null) => Promise<void>;
  setTarget: (providerId: string, accountId: string, model: string) => Promise<void>;
  refreshMessages: () => Promise<void>;
}

function emptyMessage(
  conversationId: string,
  role: "user" | "assistant",
  id: string,
  seq: number,
  content: string,
): Message {
  const now = new Date().toISOString();
  return {
    id,
    conversationId,
    seq,
    role,
    content,
    reasoning: null,
    status: role === "assistant" ? "streaming" : "complete",
    error: null,
    interrupted: false,
    interruptionReason: null,
    segments: [],
    providerId: null,
    accountId: null,
    model: null,
    inputTokens: null,
    outputTokens: null,
    createdAt: now,
    updatedAt: now,
  };
}

function patchMessage(
  messages: MessageView[],
  id: string,
  patch: Partial<Message>,
): MessageView[] {
  return messages.map((view) =>
    view.message.id === id
      ? { ...view, message: { ...view.message, ...patch } }
      : view,
  );
}

type ChatSet = (updater: (state: ChatState) => Partial<ChatState>) => void;

function reduceStreamEvent(
  event: StreamEvent,
  assistantId: string,
  set: ChatSet,
) {
  if (event.type === "delta") {
    set((current) => {
      const content = current.streaming.content + event.content;
      return {
        streaming: { ...current.streaming, content },
        messages: patchMessage(current.messages, assistantId, {
          content,
          status: "streaming",
        }),
      };
    });
  } else if (event.type === "reasoning") {
    set((current) => {
      const reasoning = current.streaming.reasoning
        ? `${current.streaming.reasoning}\n${event.content}`
        : event.content;
      return {
        streaming: { ...current.streaming, reasoning },
        messages: patchMessage(current.messages, assistantId, { reasoning }),
      };
    });
  } else if (event.type === "usage") {
    set((current) => ({
      messages: patchMessage(current.messages, assistantId, {
        inputTokens: event.inputTokens,
        outputTokens: event.outputTokens,
      }),
    }));
  } else if (event.type === "attemptStarted") {
    set((current) => ({
      streaming: {
        ...current.streaming,
        attempt: event.attemptNo,
        routing: {
          providerId: event.providerId,
          accountId: event.accountId,
          model: event.model,
        },
        statusLine: event.attemptNo > 1 ? `Retry attempt ${event.attemptNo}` : null,
      },
      messages: patchMessage(current.messages, assistantId, {
        providerId: event.providerId,
        accountId: event.accountId,
        model: event.model,
      }),
    }));
  } else if (event.type === "retryScheduled") {
    set((current) => ({
      streaming: {
        ...current.streaming,
        statusLine: `Retrying in ${Math.max(1, Math.round(event.delayMs / 1000))}s`,
      },
    }));
  } else if (event.type === "interrupted") {
    set((current) => {
      const message = current.messages.find(
        (view) => view.message.id === assistantId,
      )?.message;
      return {
        streaming: {
          ...current.streaming,
          active: false,
          error: event.reason,
          statusLine: "Interrupted",
        },
        messages: patchMessage(current.messages, assistantId, {
          content: message?.content ?? current.streaming.content,
          status: "error",
          error: event.reason,
          interrupted: true,
          interruptionReason: event.reason,
        }),
      };
    });
  } else if (event.type === "done") {
    set((current) => ({
      streaming: { ...current.streaming, statusLine: null },
    }));
  } else if (event.type === "error") {
    set((current) => ({
      streaming: { ...current.streaming, error: event.message },
    }));
    useUiStore.getState().toast(event.message, "error");
  }
}

export const useChatStore = create<ChatState>()((set, get) => ({
  conversations: [],
  activeId: null,
  messages: [],
  attachments: [],
  streaming: idleStreaming,
  loading: false,
  searchQuery: "",
  async setSearchQuery(query) {
    set({ searchQuery: query });
    await get().loadConversations({ query: query.trim() || undefined });
  },
  async loadConversations(opts) {
    set({ loading: true });
    try {
      const conversations = await api.listConversations({
        projectId: opts?.projectId,
        query: opts?.query,
        includeArchived: opts?.includeArchived ?? false,
      });
      set({ conversations, loading: false });
    } catch (error) {
      set({ loading: false });
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async openConversation(id) {
    set({ activeId: id, streaming: idleStreaming, attachments: [] });
    await get().refreshMessages();
  },
  async refreshMessages() {
    const id = get().activeId;
    if (!id) return;
    try {
      const messages = await api.listMessages(id);
      if (get().activeId === id) set({ messages });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async newConversation(projectId) {
    try {
      const selection = await useProvidersStore.getState().defaultSelection();
      const conversation = await api.createConversation({
        projectId,
        providerId: selection?.providerId,
        accountId: selection?.accountId,
        model: selection?.model,
      });
      await get().loadConversations();
      await get().openConversation(conversation.id);
      useUiStore.getState().navigate({ name: "chat" });
      return conversation;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
  async send(content) {
    const state = get();
    const conversation = state.conversations.find((item) => item.id === state.activeId);
    if (!conversation) return false;
    if (state.streaming.active) return false;
    const trimmed = content.trim();
    if (!trimmed && state.attachments.length === 0) return false;
    if (!conversation.providerId || !conversation.model) {
      useUiStore.getState().toast("Choose a provider and model first.", "error");
      return false;
    }
    const attachmentIds = state.attachments.map((item) => item.id);
    const attachments = state.attachments;

    const seq = state.messages.length
      ? Math.max(...state.messages.map((view) => view.message.seq)) + 1
      : 1;
    const temporaryUserId = `pending-user-${Date.now()}`;
    const temporaryAssistantId = `pending-assistant-${Date.now()}`;
    set({
      messages: [
        ...state.messages,
        {
          message: emptyMessage(
            conversation.id,
            "user",
            temporaryUserId,
            seq,
            trimmed,
          ),
          attachments,
        },
        {
          message: emptyMessage(
            conversation.id,
            "assistant",
            temporaryAssistantId,
            seq + 1,
            "",
          ),
          attachments: [],
        },
      ],
      attachments: [],
      streaming: {
        active: true,
        assistantId: temporaryAssistantId,
        content: "",
        reasoning: "",
        error: null,
        attempt: null,
        routing: null,
        statusLine: null,
      },
    });

    const channel = new Channel<StreamEvent>();
    let assistantId = temporaryAssistantId;
    channel.onmessage = (event) => {
      if (event.type === "started") {
        assistantId = event.messageId;
        set((current) => ({
          streaming: { ...current.streaming, assistantId: event.messageId },
          messages: current.messages.map((view) =>
            view.message.id === temporaryAssistantId
              ? {
                  ...view,
                  message: { ...view.message, id: event.messageId },
                }
              : view.message.id === temporaryUserId
                ? {
                    ...view,
                    message: { ...view.message, id: event.userMessageId },
                  }
                : view,
          ),
        }));
        return;
      }
      reduceStreamEvent(event, assistantId, set);
    };

    try {
      await api.sendMessage(conversation.id, trimmed, attachmentIds, channel);
      await get().refreshMessages();
      await get().loadConversations({ query: get().searchQuery.trim() || undefined });
      set({ streaming: idleStreaming });
      return true;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      set((current) => ({
        streaming: { ...current.streaming, active: false, error: errorMessage(error) },
      }));
      await get().refreshMessages();
      return false;
    }
  },
  async continueResponse(messageId) {
    const state = get();
    const conversation = state.conversations.find((item) => item.id === state.activeId);
    const view = state.messages.find((item) => item.message.id === messageId);
    if (!conversation || !view || state.streaming.active) return false;
    if (!conversation.providerId || !conversation.model) {
      useUiStore.getState().toast("Choose a provider and model first.", "error");
      return false;
    }
    const message = view.message;
    const channel = new Channel<StreamEvent>();
    set((current) => ({
      messages: patchMessage(current.messages, messageId, {
        status: "streaming",
        error: null,
        interrupted: false,
        interruptionReason: null,
      }),
      streaming: {
        active: true,
        assistantId: messageId,
        content: message.content,
        reasoning: message.reasoning ?? "",
        error: null,
        attempt: null,
        routing: null,
        statusLine: "Resuming response",
      },
    }));
    channel.onmessage = (event) => {
      if (event.type === "started") {
        set((current) => ({
          streaming: { ...current.streaming, assistantId: event.messageId },
        }));
        return;
      }
      reduceStreamEvent(event, messageId, set);
    };
    try {
      await api.continueGeneration(conversation.id, messageId, channel);
      await get().refreshMessages();
      await get().loadConversations({ query: get().searchQuery.trim() || undefined });
      set({ streaming: idleStreaming });
      return true;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      set((current) => ({
        streaming: {
          ...current.streaming,
          active: false,
          error: errorMessage(error),
        },
      }));
      await get().refreshMessages();
      return false;
    }
  },
  async cancel() {
    const streaming = get().streaming;
    if (!streaming.active) return;
    if (!streaming.assistantId || streaming.assistantId.startsWith("pending-")) {
      return;
    }
    try {
      const cancelled = await api.cancelStream(streaming.assistantId);
      if (!cancelled) {
        useUiStore.getState().toast("This response already finished.");
      }
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async attachFiles(paths) {
    const id = get().activeId;
    if (!id || paths.length === 0) return;
    try {
      const imported = await api.importAttachments({
        conversationId: id,
        paths,
      });
      set((state) => ({ attachments: [...state.attachments, ...imported] }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async removeAttachment(id) {
    try {
      await api.deleteAttachment(id);
      set((state) => ({
        attachments: state.attachments.filter((item) => item.id !== id),
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async renameConversation(id, title) {
    try {
      const updated = await api.renameConversation(id, title);
      set((state) => ({
        conversations: state.conversations.map((item) =>
          item.id === id ? updated : item,
        ),
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async togglePin(id) {
    const conversation = get().conversations.find((item) => item.id === id);
    if (!conversation) return;
    try {
      await api.setConversationFlags(id, { pinned: !conversation.pinned });
      await get().loadConversations({ query: get().searchQuery.trim() || undefined });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async archiveConversation(id) {
    try {
      await api.setConversationFlags(id, { archived: true });
      if (get().activeId === id) {
        set({ activeId: null, messages: [], attachments: [] });
      }
      await get().loadConversations({ query: get().searchQuery.trim() || undefined });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async deleteConversation(id) {
    try {
      await api.deleteConversation(id);
      if (get().activeId === id) {
        set({ activeId: null, messages: [], attachments: [], streaming: idleStreaming });
      }
      await get().loadConversations({ query: get().searchQuery.trim() || undefined });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async moveConversation(id, projectId) {
    try {
      const updated = await api.moveConversation(id, projectId);
      set((state) => ({
        conversations: state.conversations.map((item) =>
          item.id === id ? updated : item,
        ),
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async setTarget(providerId, accountId, model) {
    const id = get().activeId;
    if (!id) return;
    try {
      const updated = await api.setConversationTarget(id, providerId, accountId, model);
      set((state) => ({
        conversations: state.conversations.map((item) =>
          item.id === id ? updated : item,
        ),
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
}));
