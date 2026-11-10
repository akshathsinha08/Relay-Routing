import { FolderClosed, FolderPlus, MessageSquare } from "lucide-react";
import { useState } from "react";

import { relativeTime, truncate } from "@/lib/format";
import { useProjectsStore } from "@/stores/projects";
import { useUiStore } from "@/stores/ui";

export function ProjectsView() {
  const projects = useProjectsStore((state) => state.projects);
  const create = useProjectsStore((state) => state.create);
  const open = useProjectsStore((state) => state.open);
  const navigate = useUiStore((state) => state.navigate);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!name.trim() || busy) return;
    setBusy(true);
    const project = await create(name.trim(), description.trim());
    setBusy(false);
    if (project) {
      setCreating(false);
      setName("");
      setDescription("");
      await open(project.id);
      navigate({ name: "project", id: project.id });
    }
  };

  return (
    <div className="page">
      <div className="page-narrow">
        <div className="page-header">
          <div>
            <h1>Projects</h1>
            <p>Group conversations, shared files, and durable agent tasks.</p>
          </div>
          <span className="spacer" />
          <button className="btn primary" onClick={() => setCreating((value) => !value)}>
            <FolderPlus size={14} />
            New project
          </button>
        </div>

        {creating ? (
          <div className="card card-pad" style={{ marginBottom: 14 }}>
            <div className="field">
              <label htmlFor="project-name">Name</label>
              <input
                id="project-name"
                value={name}
                autoFocus
                placeholder="e.g. Release notes"
                onChange={(event) => setName(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void submit();
                }}
              />
            </div>
            <div className="field">
              <label htmlFor="project-description">Description</label>
              <input
                id="project-description"
                value={description}
                placeholder="Optional summary"
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>
            <div className="row">
              <button className="btn primary" onClick={() => void submit()} disabled={!name.trim() || busy}>
                {busy ? "Creating…" : "Create project"}
              </button>
              <button className="btn ghost" onClick={() => setCreating(false)}>
                Cancel
              </button>
            </div>
          </div>
        ) : null}

        {projects.length === 0 ? (
          <div className="card">
            <div className="empty">
              <div className="empty-icon">
                <FolderClosed size={20} />
              </div>
              <h3>No projects yet</h3>
              <p>
                Projects collect related chats and files. Tasks can also write reports
                back into a project.
              </p>
            </div>
          </div>
        ) : (
          <div className="list">
            {projects.map((project) => (
              <button
                key={project.id}
                className="list-item"
                onClick={() => {
                  void open(project.id);
                  navigate({ name: "project", id: project.id });
                }}
              >
                <FolderClosed size={16} className="faint" />
                <div className="grow">
                  <div className="title">{truncate(project.name, 60)}</div>
                  <div className="sub">
                    {project.description || "No description"} · updated{" "}
                    {relativeTime(project.updatedAt)}
                  </div>
                </div>
                <div className="right">
                  <MessageSquare size={12} />
                  {project.conversationCount}
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
