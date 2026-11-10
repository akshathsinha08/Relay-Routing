import { FolderClosed, MessageSquarePlus, Sparkles, TriangleAlert } from "lucide-react";

import { Composer } from "@/components/Composer";
import { MessageList } from "@/components/MessageList";
import { useChatStore } from "@/stores/chat";
import { useProvidersStore } from "@/stores/providers";
import { useUiStore } from "@/stores/ui";

export function ChatView() {
  const activeId = useChatStore((state) => state.activeId);
  const messages = useChatStore((state) => state.messages);
  const conversations = useChatStore((state) => state.conversations);
  const streaming = useChatStore((state) => state.streaming);
  const newConversation = useChatStore((state) => state.newConversation);
  const navigate = useUiStore((state) => state.navigate);
  const providers = useProvidersStore((state) => state.providers);
  const hasProvider = providers.some((provider) => provider.enabled);

  const conversation = conversations.find((item) => item.id === activeId);

  if (!conversation) {
    return (
      <div className="chat-view">
        <div className="empty" style={{ flex: 1 }}>
          <div className="empty-icon">
            <Sparkles size={20} />
          </div>
          <h3>Start a conversation</h3>
          <p>
            Chat with a local Ollama model or your Anthropic API account. Conversations,
            attachments, and usage stay on this Mac.
          </p>
          {!hasProvider ? (
            <p className="badge warning">
              <TriangleAlert size={11} /> No provider enabled — add one first.
            </p>
          ) : null}
          <div className="row" style={{ marginTop: 6 }}>
            <button className="btn primary" onClick={() => void newConversation()}>
              <MessageSquarePlus size={14} />
              New chat
            </button>
            <button className="btn" onClick={() => navigate({ name: "projects" })}>
              <FolderClosed size={14} />
              Browse projects
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="chat-view">
      {messages.length === 0 ? (
        <div className="empty" style={{ flex: 1 }}>
          <div className="empty-icon">
            <Sparkles size={20} />
          </div>
          <h3>What are we working on?</h3>
          <p>
            {conversation.model
              ? `Using ${conversation.model}. Ask anything, attach a file, or paste code.`
              : "Pick a model from the top bar, then ask anything."}
          </p>
        </div>
      ) : (
        <MessageList />
      )}
      {streaming.error ? (
        <div
          className="small-text"
          style={{ textAlign: "center", color: "var(--danger)", paddingBottom: 4 }}
        >
          {streaming.error}
        </div>
      ) : null}
      <Composer />
    </div>
  );
}
