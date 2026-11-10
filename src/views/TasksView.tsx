import {
  Ban,
  CircleCheck,
  CircleDot,
  CircleX,
  FileText,
  ListChecks,
  Loader2,
  Pause,
  Play,
  Plus,
  RotateCcw,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { formatBytes, formatDuration, relativeTime, taskStatusLabel } from "@/lib/format";
import { useProjectsStore } from "@/stores/projects";
import { useProvidersStore } from "@/stores/providers";
import { useTasksStore } from "@/stores/tasks";
import { useUiStore } from "@/stores/ui";
import type { Task } from "@/types";

function statusBadgeClass(status: string): string {
  switch (status) {
    case "completed":
      return "badge success";
    case "running":
      return "badge accent";
    case "paused":
    case "waiting_for_input":
      return "badge warning";
    case "failed":
    case "cancelled":
      return "badge danger";
    default:
      return "badge";
  }
}

function stepIcon(status: string, seq: number) {
  if (status === "completed") return <CircleCheck size={13} />;
  if (status === "running") return <Loader2 size={13} className="spin" />;
  if (status === "failed") return <CircleX size={13} />;
  if (status === "paused") return <Pause size={12} />;
  return <span>{seq}</span>;
}

export function TasksView() {
  const tasks = useTasksStore((state) => state.tasks);
  const detail = useTasksStore((state) => state.detail);
  const busy = useTasksStore((state) => state.busy);
  const load = useTasksStore((state) => state.load);
  const open = useTasksStore((state) => state.open);
  const stopPolling = useTasksStore((state) => state.stopPolling);
  const create = useTasksStore((state) => state.create);
  const start = useTasksStore((state) => state.start);
  const pause = useTasksStore((state) => state.pause);
  const cancel = useTasksStore((state) => state.cancel);
  const resume = useTasksStore((state) => state.resume);
  const remove = useTasksStore((state) => state.remove);
  const openArtifact = useTasksStore((state) => state.openArtifact);
  const projects = useProjectsStore((state) => state.projects);
  const accounts = useProvidersStore((state) => state.accounts);
  const providers = useProvidersStore((state) => state.providers);
  const cachedModels = useProvidersStore((state) => state.cachedModels);
  const ensureModels = useProvidersStore((state) => state.ensureModels);
  const toast = useUiStore((state) => state.toast);

  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [instructions, setInstructions] = useState("");
  const [projectId, setProjectId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [model, setModel] = useState("");
  const [creatingBusy, setCreatingBusy] = useState(false);

  useEffect(() => {
    void load();
    return () => stopPolling();
  }, [load, stopPolling]);

  const selectedAccount = accounts.find((account) => account.id === accountId);
  const models = selectedAccount
    ? cachedModels(selectedAccount.providerId, selectedAccount.id)
    : [];

  useEffect(() => {
    if (!selectedAccount) return;
    if (cachedModels(selectedAccount.providerId, selectedAccount.id).length > 0) return;
    void ensureModels(selectedAccount.providerId, selectedAccount.id);
  }, [selectedAccount, cachedModels, ensureModels]);

  useEffect(() => {
    if (!selectedAccount) return;
    if (model && models.some((item) => item.id === model)) return;
    const first = models.find((item) => item.capabilities.includes("completion")) ?? models[0];
    setModel(first?.id ?? "");
  }, [selectedAccount, models, model]);

  const accountLabel = useMemo(() => {
    const map = new Map<string, string>();
    for (const account of accounts) {
      const provider = providers.find((item) => item.id === account.providerId);
      map.set(account.id, `${provider?.name ?? account.providerId} · ${account.label}`);
    }
    return map;
  }, [accounts, providers]);

  const submit = async () => {
    if (!name.trim() || !instructions.trim() || !selectedAccount || !model) {
      toast("Name, instructions, account, and model are required.", "error");
      return;
    }
    setCreatingBusy(true);
    const task = await create({
      name: name.trim(),
      instructions: instructions.trim(),
      projectId: projectId || null,
      providerId: selectedAccount.providerId,
      accountId: selectedAccount.id,
      model,
    });
    setCreatingBusy(false);
    if (task) {
      setCreating(false);
      setName("");
      setInstructions("");
      void open(task.id);
    }
  };

  const activeTask = detail?.task ?? null;

  return (
    <div className="page">
      <div className="page-narrow">
        <div className="page-header">
          <div>
            <h1>Tasks</h1>
            <p>
              Durable agent runs: a plan, sequential steps, checkpoints, and a final
              Markdown report.
            </p>
          </div>
          <span className="spacer" />
          <button className="btn primary" onClick={() => setCreating((value) => !value)}>
            <Plus size={14} />
            New task
          </button>
        </div>

        {creating ? (
          <div className="card card-pad" style={{ marginBottom: 14 }}>
            <div className="field">
              <label htmlFor="task-name">Task name</label>
              <input
                id="task-name"
                value={name}
                autoFocus
                placeholder="e.g. Summarize Q4 planning notes"
                onChange={(event) => setName(event.target.value)}
              />
            </div>
            <div className="field">
              <label htmlFor="task-instructions">Instructions</label>
              <textarea
                id="task-instructions"
                value={instructions}
                placeholder="Describe the outcome you want. The agent will produce a plan, then execute each step."
                onChange={(event) => setInstructions(event.target.value)}
              />
            </div>
            <div className="row wrap" style={{ gap: 10, alignItems: "flex-end" }}>
              <div className="field" style={{ minWidth: 180, marginBottom: 0 }}>
                <label htmlFor="task-project">Project</label>
                <select
                  id="task-project"
                  value={projectId}
                  onChange={(event) => setProjectId(event.target.value)}
                >
                  <option value="">No project</option>
                  {projects.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="field" style={{ minWidth: 200, marginBottom: 0 }}>
                <label htmlFor="task-account">Account</label>
                <select
                  id="task-account"
                  value={accountId}
                  onChange={(event) => setAccountId(event.target.value)}
                >
                  <option value="">Choose account</option>
                  {accounts
                    .filter((account) => {
                      const provider = providers.find(
                        (item) => item.id === account.providerId,
                      );
                      return provider?.enabled;
                    })
                    .map((account) => (
                      <option key={account.id} value={account.id}>
                        {accountLabel.get(account.id) ?? account.label}
                      </option>
                    ))}
                </select>
              </div>
              <div className="field" style={{ minWidth: 200, marginBottom: 0 }}>
                <label htmlFor="task-model">Model</label>
                <select
                  id="task-model"
                  value={model}
                  onChange={(event) => setModel(event.target.value)}
                  disabled={!selectedAccount}
                >
                  <option value="">Choose model</option>
                  {models.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.id}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div className="row" style={{ marginTop: 14 }}>
              <button
                className="btn primary"
                onClick={() => void submit()}
                disabled={
                  creatingBusy ||
                  !name.trim() ||
                  !instructions.trim() ||
                  !selectedAccount ||
                  !model
                }
              >
                {creatingBusy ? "Creating…" : "Create task"}
              </button>
              <button className="btn ghost" onClick={() => setCreating(false)}>
                Cancel
              </button>
            </div>
          </div>
        ) : null}

        <div className="row" style={{ alignItems: "flex-start", gap: 16 }}>
          <div className="grow" style={{ minWidth: 0 }}>
            {tasks.length === 0 ? (
              <div className="card">
                <div className="empty">
                  <div className="empty-icon">
                    <ListChecks size={20} />
                  </div>
                  <h3>No tasks yet</h3>
                  <p>
                    Create a task, start it, and the agent will plan and execute steps
                    with pause, resume, and cancel support.
                  </p>
                </div>
              </div>
            ) : (
              <div className="list">
                {tasks.map((task: Task) => (
                  <button
                    key={task.id}
                    className="list-item"
                    onClick={() => void open(task.id)}
                    style={
                      activeTask?.id === task.id
                        ? { background: "var(--bg-hover)" }
                        : undefined
                    }
                  >
                    <CircleDot size={14} className="faint" />
                    <div className="grow">
                      <div className="title">{task.name}</div>
                      <div className="sub">
                        {task.projectName ?? "No project"} · step {task.currentStep} ·{" "}
                        {relativeTime(task.updatedAt)}
                      </div>
                    </div>
                    <span className={statusBadgeClass(task.status)}>
                      {taskStatusLabel(task.status)}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {detail ? (
            <div className="grow" style={{ minWidth: 0, maxWidth: 520 }}>
              <div className="card card-pad">
                <div className="row">
                  <h2 className="grow" style={{ margin: 0 }}>
                    {detail.task.name}
                  </h2>
                  <span className={statusBadgeClass(detail.task.status)}>
                    {taskStatusLabel(detail.task.status)}
                  </span>
                </div>
                <p className="small-text" style={{ marginTop: 6 }}>
                  {detail.task.instructions}
                </p>
                <div className="stat-grid" style={{ margin: "12px 0" }}>
                  <div className="stat">
                    <div className="label">Steps</div>
                    <div className="value">
                      {detail.steps.filter((step) => step.status === "completed").length}/
                      {detail.steps.length || "—"}
                    </div>
                  </div>
                  <div className="stat">
                    <div className="label">Runtime</div>
                    <div className="value">
                      {formatDuration(detail.task.startedAt, detail.task.completedAt)}
                    </div>
                  </div>
                  <div className="stat">
                    <div className="label">Retries</div>
                    <div className="value">{detail.task.retryCount}</div>
                  </div>
                </div>

                <div className="row wrap">
                  {["queued", "paused", "failed"].includes(detail.task.status) ? (
                    <button
                      className="btn primary small"
                      disabled={busy}
                      onClick={() => void start(detail.task.id)}
                    >
                      <Play size={12} />
                      {detail.task.status === "paused" ? "Resume" : "Start"}
                    </button>
                  ) : null}
                  {detail.task.status === "running" ? (
                    <button
                      className="btn small"
                      disabled={busy}
                      onClick={() => void pause(detail.task.id)}
                    >
                      <Pause size={12} />
                      Pause
                    </button>
                  ) : null}
                  {["queued", "running", "paused", "waiting_for_input"].includes(
                    detail.task.status,
                  ) ? (
                    <button
                      className="btn small danger"
                      disabled={busy}
                      onClick={() => void cancel(detail.task.id)}
                    >
                      <Ban size={12} />
                      Cancel
                    </button>
                  ) : null}
                  {detail.task.status === "failed" ? (
                    <button
                      className="btn small"
                      disabled={busy}
                      onClick={() => void resume(detail.task.id)}
                    >
                      <RotateCcw size={12} />
                      Retry
                    </button>
                  ) : null}
                  <span className="grow" />
                  <button
                    className="icon-btn"
                    title="Delete task"
                    onClick={() => {
                      if (confirm(`Delete task "${detail.task.name}"?`)) {
                        void remove(detail.task.id);
                      }
                    }}
                  >
                    <Trash2 size={13} />
                  </button>
                </div>

                {detail.task.error ? (
                  <div className="message-error" style={{ marginTop: 10 }}>
                    {detail.task.error}
                  </div>
                ) : null}

                <div className="progress" style={{ marginTop: 12 }}>
                  <span
                    style={{
                      width: `${
                        detail.steps.length
                          ? Math.round(
                              (detail.steps.filter((step) => step.status === "completed")
                                .length /
                                detail.steps.length) *
                                100,
                            )
                          : 0
                      }%`,
                    }}
                  />
                </div>
              </div>

              <div className="card card-pad">
                <h3>Plan</h3>
                {detail.steps.length === 0 ? (
                  <p className="faint small-text">
                    No plan yet. Start the task to generate one.
                  </p>
                ) : (
                  <div className="steps">
                    {detail.steps.map((step) => (
                      <div key={step.id} className={`step ${step.status}`}>
                        <div className="step-icon">{stepIcon(step.status, step.seq)}</div>
                        <div className="grow">
                          <div className="step-title">{step.title}</div>
                          <div className="step-instruction">{step.instruction}</div>
                          {step.output ? (
                            <pre className="step-output">{step.output}</pre>
                          ) : null}
                          {step.error ? (
                            <div className="message-error">{step.error}</div>
                          ) : null}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {detail.artifacts.length > 0 ? (
                <div className="card card-pad">
                  <h3>Artifacts</h3>
                  <div className="list">
                    {detail.artifacts.map((artifact) => (
                      <button
                        key={artifact.id}
                        className="list-item"
                        onClick={() => void openArtifact(artifact.id)}
                      >
                        <FileText size={14} />
                        <div className="grow">
                          <div className="title">{artifact.name}</div>
                          <div className="sub">
                            {formatBytes(artifact.size)} ·{" "}
                            {relativeTime(artifact.createdAt)}
                          </div>
                        </div>
                      </button>
                    ))}
                  </div>
                </div>
              ) : null}

              {detail.logs.length > 0 ? (
                <details className="section" style={{ marginTop: 12 }}>
                  <summary>Execution log ({detail.logs.length})</summary>
                  <div>
                    <pre className="step-output" style={{ maxHeight: 260 }}>
                      {detail.logs
                        .slice(-30)
                        .map(
                          (log) =>
                            `${new Date(log.createdAt).toLocaleTimeString()} [${log.level}] ${log.message}`,
                        )
                        .join("\n")}
                    </pre>
                  </div>
                </details>
              ) : null}

              <div className="faint small-text" style={{ marginTop: 10 }}>
                {detail.checkpoints.length} checkpoint
                {detail.checkpoints.length === 1 ? "" : "s"} saved
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
