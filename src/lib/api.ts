import { invoke, type Channel } from "@tauri-apps/api/core";

import type {
  Account,
  AccountTest,
  AppInfo,
  Artifact,
  Attachment,
  BackupInfo,
  Conversation,
  Message,
  MessageView,
  ModelInfo,
  Project,
  Provider,
  ProviderHealth,
  SearchHit,
  StreamEvent,
  Task,
  TaskDetail,
  UsageSummary,
} from "@/types";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  healthCheck: () => invoke<void>("health_check"),
  backupDatabase: () => invoke<BackupInfo>("backup_database"),
  openDataDir: () => invoke<void>("open_data_dir"),

  listProviders: () => invoke<Provider[]>("list_providers"),
  updateProvider: (
    id: string,
    patch: { name?: string; baseUrl?: string; enabled?: boolean },
  ) => invoke<Provider>("update_provider", { id, ...patch }),
  listAccounts: (providerId?: string) =>
    invoke<Account[]>("list_accounts", { providerId: providerId ?? null }),
  createAccount: (
    providerId: string,
    label: string,
    authKind: string,
    secret?: string,
  ) =>
    invoke<Account>("create_account", {
      providerId,
      label,
      authKind,
      secret: secret ?? null,
    }),
  updateAccountSecret: (accountId: string, secret: string) =>
    invoke<Account>("update_account_secret", { accountId, secret }),
  renameAccount: (accountId: string, label: string) =>
    invoke<Account>("rename_account", { accountId, label }),
  deleteAccount: (accountId: string) =>
    invoke<void>("delete_account", { accountId }),
  providerHealth: (providerId: string, accountId?: string) =>
    invoke<ProviderHealth>("provider_health", {
      providerId,
      accountId: accountId ?? null,
    }),
  testAccount: (accountId: string) =>
    invoke<AccountTest>("test_account", { accountId }),
  listModels: (providerId: string, accountId?: string) =>
    invoke<ModelInfo[]>("list_models", {
      providerId,
      accountId: accountId ?? null,
    }),

  listProjects: (includeArchived = false) =>
    invoke<Project[]>("list_projects", { includeArchived }),
  getProject: (id: string) => invoke<Project>("get_project", { id }),
  createProject: (name: string, description = "", instructions = "") =>
    invoke<Project>("create_project", { name, description, instructions }),
  updateProject: (
    id: string,
    patch: {
      name?: string;
      description?: string;
      instructions?: string;
      archived?: boolean;
    },
  ) => invoke<Project>("update_project", { id, ...patch }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),
  listProjectAttachments: (projectId: string) =>
    invoke<Attachment[]>("list_project_attachments", { projectId }),
  listProjectArtifacts: (projectId: string) =>
    invoke<Artifact[]>("list_project_artifacts", { projectId }),

  listConversations: (opts?: {
    projectId?: string;
    includeArchived?: boolean;
    query?: string;
  }) =>
    invoke<Conversation[]>("list_conversations", {
      projectId: opts?.projectId ?? null,
      includeArchived: opts?.includeArchived ?? false,
      query: opts?.query ?? null,
    }),
  getConversation: (id: string) =>
    invoke<Conversation>("get_conversation", { id }),
  createConversation: (opts?: {
    projectId?: string;
    title?: string;
    providerId?: string;
    accountId?: string;
    model?: string;
  }) =>
    invoke<Conversation>("create_conversation", {
      projectId: opts?.projectId ?? null,
      title: opts?.title ?? null,
      providerId: opts?.providerId ?? null,
      accountId: opts?.accountId ?? null,
      model: opts?.model ?? null,
    }),
  renameConversation: (id: string, title: string) =>
    invoke<Conversation>("rename_conversation", { id, title }),
  setConversationFlags: (
    id: string,
    flags: { pinned?: boolean; archived?: boolean },
  ) => invoke<Conversation>("set_conversation_flags", { id, ...flags }),
  setConversationTarget: (
    id: string,
    providerId: string,
    accountId: string | null,
    model: string,
  ) =>
    invoke<Conversation>("set_conversation_target", {
      id,
      providerId,
      accountId,
      model,
    }),
  moveConversation: (id: string, projectId: string | null) =>
    invoke<Conversation>("move_conversation", { id, projectId }),
  deleteConversation: (id: string) =>
    invoke<void>("delete_conversation", { id }),
  listMessages: (conversationId: string) =>
    invoke<MessageView[]>("list_messages", { conversationId }),
  searchMessages: (query: string, limit = 50) =>
    invoke<SearchHit[]>("search_messages", { query, limit }),
  exportConversation: (conversationId: string, destination: string) =>
    invoke<string>("export_conversation", { conversationId, destination }),
  sendMessage: (
    conversationId: string,
    content: string,
    attachmentIds: string[],
    channel: Channel<StreamEvent>,
  ) =>
    invoke<Message>("send_message", {
      conversationId,
      content,
      attachmentIds,
      channel,
    }),
  continueGeneration: (
    conversationId: string,
    messageId: string,
    channel: Channel<StreamEvent>,
  ) =>
    invoke<Message>("continue_generation", {
      conversationId,
      messageId,
      channel,
    }),
  cancelStream: (messageId: string) =>
    invoke<boolean>("cancel_stream", { messageId }),

  importAttachments: (opts: {
    projectId?: string;
    conversationId?: string;
    paths: string[];
  }) =>
    invoke<Attachment[]>("import_attachments", {
      projectId: opts.projectId ?? null,
      conversationId: opts.conversationId ?? null,
      paths: opts.paths,
    }),
  deleteAttachment: (id: string) =>
    invoke<void>("delete_attachment", { id }),
  attachmentPreview: (id: string) =>
    invoke<string | null>("attachment_preview", { id }),
  openAttachment: (id: string) => invoke<void>("open_attachment", { id }),

  listTasks: (projectId?: string, limit = 100) =>
    invoke<Task[]>("list_tasks", { projectId: projectId ?? null, limit }),
  getTaskDetail: (taskId: string) =>
    invoke<TaskDetail>("get_task_detail", { taskId }),
  createTask: (payload: {
    name: string;
    instructions: string;
    projectId?: string | null;
    conversationId?: string | null;
    providerId?: string | null;
    accountId?: string | null;
    model?: string | null;
  }) =>
    invoke<Task>("create_task", {
      name: payload.name,
      instructions: payload.instructions,
      projectId: payload.projectId ?? null,
      conversationId: payload.conversationId ?? null,
      providerId: payload.providerId ?? null,
      accountId: payload.accountId ?? null,
      model: payload.model ?? null,
    }),
  startTask: (taskId: string) => invoke<Task>("start_task", { taskId }),
  pauseTask: (taskId: string) => invoke<Task>("pause_task", { taskId }),
  cancelTask: (taskId: string) => invoke<Task>("cancel_task", { taskId }),
  resumeTask: (taskId: string) => invoke<Task>("resume_task", { taskId }),
  deleteTask: (taskId: string) => invoke<void>("delete_task", { taskId }),
  taskArtifacts: (taskId: string) =>
    invoke<Artifact[]>("task_artifacts", { taskId }),
  openArtifact: (id: string) => invoke<void>("open_artifact", { id }),

  getSettings: () => invoke<Record<string, string>>("get_settings"),
  updateSettings: (entries: Record<string, string>) =>
    invoke<Record<string, string>>("update_settings", { entries }),
  usageSummary: () => invoke<UsageSummary[]>("usage_summary"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object") {
    const candidate = error as { message?: unknown; code?: unknown };
    if (typeof candidate.message === "string" && candidate.message.trim()) {
      return candidate.message;
    }
    if (typeof candidate.code === "string") return candidate.code;
  }
  return "Something went wrong.";
}
