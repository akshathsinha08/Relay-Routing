import { save } from "@tauri-apps/plugin-dialog";
import {
  Archive,
  Download,
  FolderClosed,
  ListChecks,
  Pin,
  PinOff,
  Server,
  Settings,
  SquarePen,
  Trash2,
} from "lucide-react";
import { useEffect, useState } from "react";

import { api, errorMessage } from "@/lib/api";
import { conversationTitle, truncate } from "@/lib/format";
import { useChatStore } from "@/stores/chat";
import { useProjectsStore } from "@/stores/projects";
import { useUiStore } from "@/stores/ui";

import { ModelPicker } from "./ModelPicker";

export function TopBar() {
  const view = useUiStore((state) => state.view);
  const navigate = useUiStore((state) => state.navigate);
  const toast = useUiStore((state) => state.toast);
  const conversation = useChatStore((state) =>
    state.conversations.find((item) => item.id === state.activeId),
  );
  const newConversation = useChatStore((state) => state.newConversation);
  const renameConversation = useChatStore((state) => state.renameConversation);
  const togglePin = useChatStore((state) => state.togglePin);
  const archiveConversation = useChatStore((state) => state.archiveConversation);
  const deleteConversation = useChatStore((state) => state.deleteConversation);
  const activeProject = useProjectsStore((state) => state.active);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");

  useEffect(() => {
    setEditing(false);
  }, [conversation?.id]);

  const commitRename = async () => {
    if (!conversation) return;
    const next = draft.trim();
    setEditing(false);
    if (next && next !== conversation.title) {
      await renameConversation(conversation.id, next);
    }
  };

  const exportChat = async () => {
    if (!conversation) return;
    try {
      const destination = await save({
        title: "Export conversation",
        defaultPath: `${conversationTitle(conversation).replace(/[^\w\- ]+/g, "")}.md`,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (!destination) return;
      const path = await api.exportConversation(conversation.id, destination);
      toast(`Exported to ${path}`, "success");
    } catch (error) {
      toast(errorMessage(error), "error");
    }
  };

  const title = (() => {
    switch (view.name) {
      case "chat":
        return conversation ? conversationTitle(conversation) : "Chat";
      case "projects":
        return "Projects";
      case "project":
        return activeProject?.name ?? "Project";
      case "tasks":
        return "Tasks";
      case "providers":
        return "Providers";
      case "settings":
        return "Settings";
      case "onboarding":
        return "Welcome";
    }
  })();

  const icon = (() => {
    switch (view.name) {
      case "chat":
        return <SquarePen size={15} />;
      case "projects":
      case "project":
        return <FolderClosed size={15} />;
      case "tasks":
        return <ListChecks size={15} />;
      case "providers":
        return <Server size={15} />;
      case "settings":
        return <Settings size={15} />;
      default:
        return null;
    }
  })();

  return (
    <header className="topbar" data-tauri-drag-region>
      <span style={{ color: "var(--text-faint)", display: "flex" }}>{icon}</span>
      {editing && conversation ? (
        <input
          className="grow"
          style={{ maxWidth: 420 }}
          autoFocus
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={() => void commitRename()}
          onKeyDown={(event) => {
            if (event.key === "Enter") void commitRename();
            if (event.key === "Escape") setEditing(false);
          }}
        />
      ) : (
        <span className="topbar-title" style={{ maxWidth: 420 }}>
          {truncate(title, 70)}
        </span>
      )}
      <div className="topbar-spacer" />

      {view.name === "chat" && conversation ? (
        <>
          <ModelPicker />
          <div className="topbar-actions">
            <button
              className="icon-btn"
              title="Rename conversation"
              onClick={() => {
                setDraft(conversation.title);
                setEditing(true);
              }}
            >
              <SquarePen size={14} />
            </button>
            <button
              className="icon-btn"
              title={conversation.pinned ? "Unpin" : "Pin"}
              onClick={() => void togglePin(conversation.id)}
            >
              {conversation.pinned ? <PinOff size={14} /> : <Pin size={14} />}
            </button>
            <button
              className="icon-btn"
              title="Export as Markdown"
              onClick={() => void exportChat()}
            >
              <Download size={14} />
            </button>
            <button
              className="icon-btn"
              title="Archive"
              onClick={() => void archiveConversation(conversation.id)}
            >
              <Archive size={14} />
            </button>
            <button
              className="icon-btn"
              title="Delete"
              onClick={() => {
                if (confirm("Delete this conversation and its messages?")) {
                  void deleteConversation(conversation.id);
                }
              }}
            >
              <Trash2 size={14} />
            </button>
          </div>
        </>
      ) : null}

      {view.name !== "chat" && view.name !== "onboarding" ? (
        <div className="topbar-actions">
          <button className="btn small" onClick={() => void newConversation()}>
            <SquarePen size={12} />
            New chat
          </button>
          <button
            className="icon-btn"
            title="Settings"
            onClick={() => navigate({ name: "settings" })}
          >
            <Settings size={14} />
          </button>
        </div>
      ) : null}
    </header>
  );
}
