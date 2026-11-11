import {
  ListChecks,
  MessageSquare,
  Search,
  Server,
  Settings,
  SquarePen,
  FolderClosed,
  Pin,
} from "lucide-react";

import logo from "@/assets/logo.png";
import { conversationTitle, relativeTime, truncate } from "@/lib/format";
import { useChatStore } from "@/stores/chat";
import { useProjectsStore } from "@/stores/projects";
import { useTasksStore } from "@/stores/tasks";
import { useUiStore } from "@/stores/ui";

export function Sidebar() {
  const view = useUiStore((state) => state.view);
  const navigate = useUiStore((state) => state.navigate);
  const conversations = useChatStore((state) => state.conversations);
  const activeId = useChatStore((state) => state.activeId);
  const openConversation = useChatStore((state) => state.openConversation);
  const newConversation = useChatStore((state) => state.newConversation);
  const searchQuery = useChatStore((state) => state.searchQuery);
  const setSearchQuery = useChatStore((state) => state.setSearchQuery);
  const projects = useProjectsStore((state) => state.projects);
  const tasks = useTasksStore((state) => state.tasks);

  const pinned = conversations.filter((item) => item.pinned);
  const recent = conversations.filter((item) => !item.pinned);
  const activeTasks = tasks.filter((task) =>
    ["queued", "running", "waiting_for_input"].includes(task.status),
  ).length;

  const conversationButton = (id: string, title: string, subtitle: string, isPinned: boolean) => (
    <button
      key={id}
      className={`conv-item ${view.name === "chat" && activeId === id ? "active" : ""}`}
      onClick={() => {
        void openConversation(id);
        navigate({ name: "chat" });
      }}
    >
      <span className="conv-title">
        {isPinned ? <Pin size={10} /> : null}
        {truncate(title, 40)}
      </span>
      <span className="conv-sub">{subtitle}</span>
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <img className="brand-mark" src={logo} alt="" />
        <span>Relay Routing</span>
      </div>

      <div className="sidebar-actions">
        <button
          className="side-nav"
          onClick={() => {
            void newConversation();
          }}
        >
          <SquarePen size={15} />
          New chat
          <span className="count">⌘N</span>
        </button>
        <button
          className={`side-nav ${view.name === "projects" || view.name === "project" ? "active" : ""}`}
          onClick={() => navigate({ name: "projects" })}
        >
          <FolderClosed size={15} />
          Projects
          <span className="count">{projects.length || ""}</span>
        </button>
        <button
          className={`side-nav ${view.name === "tasks" ? "active" : ""}`}
          onClick={() => navigate({ name: "tasks" })}
        >
          <ListChecks size={15} />
          Tasks
          <span className="count">{activeTasks || ""}</span>
        </button>
        <button
          className={`side-nav ${view.name === "providers" ? "active" : ""}`}
          onClick={() => navigate({ name: "providers" })}
        >
          <Server size={15} />
          Providers
        </button>
      </div>

      <div className="sidebar-search">
        <Search size={13} />
        <input
          value={searchQuery}
          placeholder="Search conversations"
          onChange={(event) => {
            void setSearchQuery(event.target.value);
          }}
          aria-label="Search conversations"
        />
      </div>

      <div className="sidebar-scroll">
        {pinned.length > 0 ? (
          <>
            <div className="side-section-label">Pinned</div>
            {pinned.map((conversation) =>
              conversationButton(
                conversation.id,
                conversationTitle(conversation),
                conversation.projectName ?? relativeTime(conversation.lastMessageAt),
                true,
              ),
            )}
          </>
        ) : null}
        <div className="side-section-label">
          {searchQuery.trim() ? "Results" : "Recent"}
        </div>
        {recent.length === 0 && !searchQuery.trim() ? (
          <div className="faint small-text" style={{ padding: "4px 9px" }}>
            No conversations yet.
          </div>
        ) : (
          recent.map((conversation) =>
            conversationButton(
              conversation.id,
              conversationTitle(conversation),
              conversation.projectName ?? relativeTime(conversation.lastMessageAt),
              false,
            ),
          )
        )}
      </div>

      <div className="sidebar-footer">
        <button
          className="side-nav"
          onClick={() => navigate({ name: "settings" })}
          aria-label="Settings"
        >
          <Settings size={15} />
          Settings
        </button>
        <button
          className="side-nav"
          onClick={() => navigate({ name: "chat" })}
          aria-label="Chat"
        >
          <MessageSquare size={15} />
          Chat
        </button>
      </div>
    </aside>
  );
}
