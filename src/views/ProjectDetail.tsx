import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  ArrowLeft,
  FileText,
  MessageSquarePlus,
  Paperclip,
  Save,
  Trash2,
  Upload,
} from "lucide-react";
import { useEffect, useState } from "react";

import { api, errorMessage } from "@/lib/api";
import { formatBytes, relativeTime, truncate } from "@/lib/format";
import { useChatStore } from "@/stores/chat";
import { useProjectsStore } from "@/stores/projects";
import { useTasksStore } from "@/stores/tasks";
import { useUiStore } from "@/stores/ui";
import type { Attachment, Project } from "@/types";

export function ProjectDetail({ id }: { id: string }) {
  const active = useProjectsStore((state) => state.active);
  const attachments = useProjectsStore((state) => state.attachments);
  const artifacts = useProjectsStore((state) => state.artifacts);
  const open = useProjectsStore((state) => state.open);
  const update = useProjectsStore((state) => state.update);
  const remove = useProjectsStore((state) => state.remove);
  const importFiles = useProjectsStore((state) => state.importFiles);
  const deleteAttachment = useProjectsStore((state) => state.deleteAttachment);
  const refreshDetail = useProjectsStore((state) => state.refreshDetail);
  const conversations = useChatStore((state) => state.conversations);
  const newConversation = useChatStore((state) => state.newConversation);
  const openConversation = useChatStore((state) => state.openConversation);
  const tasks = useTasksStore((state) => state.tasks);
  const navigate = useUiStore((state) => state.navigate);
  const toast = useUiStore((state) => state.toast);

  const [draft, setDraft] = useState<Project | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!active || active.id !== id) void open(id);
  }, [id, active, open]);

  useEffect(() => {
    if (active?.id === id) setDraft(active);
  }, [active, id]);

  const project = active?.id === id ? active : null;
  const projectConversations = conversations.filter((item) => item.projectId === id);
  const projectTasks = tasks.filter((task) => task.projectId === id);

  const attach = async () => {
    try {
      const picked = await openDialog({ multiple: true, title: "Attach files" });
      if (!picked) return;
      await importFiles(Array.isArray(picked) ? picked : [picked]);
    } catch {
      // Dismissal is not an error.
    }
  };

  const save = async () => {
    if (!draft || !project) return;
    setBusy(true);
    const ok = await update(project.id, {
      name: draft.name,
      description: draft.description,
      instructions: draft.instructions,
    });
    setBusy(false);
    if (ok) toast("Project saved.", "success");
  };

  if (!project || !draft) {
    return (
      <div className="page">
        <div className="page-narrow">
          <div className="empty">Loading project…</div>
        </div>
      </div>
    );
  }

  const attachmentRow = (attachment: Attachment) => (
    <div className="list-item" key={attachment.id}>
      {attachment.kind === "image" ? <Paperclip size={14} /> : <FileText size={14} />}
      <div className="grow">
        <div className="title">{attachment.name}</div>
        <div className="sub">
          {formatBytes(attachment.size)} · {attachment.mime || "file"}
        </div>
      </div>
      <div className="right">
        <button
          className="btn small ghost"
          onClick={() => {
            void api.openAttachment(attachment.id).catch((error: unknown) => {
              toast(errorMessage(error), "error");
            });
          }}
        >
          Open
        </button>
        <button
          className="icon-btn"
          title="Remove attachment"
          onClick={() => {
            if (confirm(`Remove ${attachment.name} from this project?`)) {
              void deleteAttachment(attachment.id);
            }
          }}
        >
          <Trash2 size={13} />
        </button>
      </div>
    </div>
  );

  return (
    <div className="page">
      <div className="page-narrow">
        <div className="row" style={{ marginBottom: 14 }}>
          <button className="btn ghost small" onClick={() => navigate({ name: "projects" })}>
            <ArrowLeft size={12} />
            Projects
          </button>
          <span className="grow" />
          <button
            className="btn small"
            onClick={() => {
              if (confirm(`Delete "${project.name}"? Conversations move to no project.`)) {
                void remove(project.id).then(() => navigate({ name: "projects" }));
              }
            }}
          >
            <Trash2 size={12} />
            Delete project
          </button>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <div className="field">
            <label htmlFor="detail-name">Name</label>
            <input
              id="detail-name"
              value={draft.name}
              onChange={(event) => setDraft({ ...draft, name: event.target.value })}
            />
          </div>
          <div className="field">
            <label htmlFor="detail-description">Description</label>
            <input
              id="detail-description"
              value={draft.description}
              onChange={(event) => setDraft({ ...draft, description: event.target.value })}
            />
          </div>
          <div className="field">
            <label htmlFor="detail-instructions">Project instructions</label>
            <textarea
              id="detail-instructions"
              value={draft.instructions}
              placeholder="Persistent guidance included with every conversation in this project."
              onChange={(event) => setDraft({ ...draft, instructions: event.target.value })}
            />
            <span className="hint">
              Sent as extra context to the model for chats in this project.
            </span>
          </div>
          <div className="row">
            <button className="btn primary" onClick={() => void save()} disabled={busy}>
              <Save size={13} />
              {busy ? "Saving…" : "Save"}
            </button>
            <span className="faint small-text">
              Created {relativeTime(project.createdAt)}
            </span>
          </div>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <div className="row between" style={{ marginBottom: 10 }}>
            <h2 style={{ margin: 0 }}>Conversations</h2>
            <button
              className="btn small"
              onClick={() => {
                void newConversation(project.id);
              }}
            >
              <MessageSquarePlus size={12} />
              New chat
            </button>
          </div>
          {projectConversations.length === 0 ? (
            <p className="faint small-text" style={{ margin: 0 }}>
              No conversations yet.
            </p>
          ) : (
            <div className="list">
              {projectConversations.map((conversation) => (
                <button
                  key={conversation.id}
                  className="list-item"
                  onClick={() => {
                    void openConversation(conversation.id);
                    navigate({ name: "chat" });
                  }}
                >
                  <div className="grow">
                    <div className="title">{truncate(conversation.title, 70)}</div>
                    <div className="sub">
                      {conversation.model ?? "No model"} ·{" "}
                      {relativeTime(conversation.lastMessageAt ?? conversation.updatedAt)}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <div className="row between" style={{ marginBottom: 10 }}>
            <h2 style={{ margin: 0 }}>Shared files</h2>
            <button className="btn small" onClick={() => void attach()}>
              <Upload size={12} />
              Attach files
            </button>
          </div>
          {attachments.length === 0 ? (
            <p className="faint small-text" style={{ margin: 0 }}>
              Attach text or image files to include with chats and tasks in this project.
            </p>
          ) : (
            <div className="list">{attachments.map(attachmentRow)}</div>
          )}
        </div>

        <div className="card card-pad">
          <div className="row between" style={{ marginBottom: 10 }}>
            <h2 style={{ margin: 0 }}>Tasks & artifacts</h2>
            <button className="btn small" onClick={() => navigate({ name: "tasks" })}>
              Task board
            </button>
          </div>
          {projectTasks.length === 0 ? (
            <p className="faint small-text" style={{ margin: 0 }}>
              No tasks in this project.
            </p>
          ) : (
            <div className="list">
              {projectTasks.map((task) => (
                <button
                  key={task.id}
                  className="list-item"
                  onClick={() => navigate({ name: "tasks" })}
                >
                  <div className="grow">
                    <div className="title">{task.name}</div>
                    <div className="sub">
                      {task.status} · step {task.currentStep}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
          {artifacts.length > 0 ? (
            <>
              <div className="divider" />
              <h3>Artifacts</h3>
              <div className="list">
                {artifacts.map((artifact) => (
                  <button
                    key={artifact.id}
                    className="list-item"
                    onClick={() => {
                      void api.openArtifact(artifact.id).catch((error: unknown) => {
                        toast(errorMessage(error), "error");
                      });
                    }}
                  >
                    <FileText size={14} />
                    <div className="grow">
                      <div className="title">{artifact.name}</div>
                      <div className="sub">
                        {formatBytes(artifact.size)} · {relativeTime(artifact.createdAt)}
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            </>
          ) : null}
          <div style={{ marginTop: 8 }}>
            <button className="btn small ghost" onClick={() => void refreshDetail()}>
              Refresh project data
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
