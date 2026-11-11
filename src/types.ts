export interface AppInfo {
  version: string;
  identifier: string;
  platform: string;
  dataDir: string;
  dbPath: string;
  logDir: string;
  artifactsDir: string;
}

export interface Provider {
  id: string;
  kind: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Account {
  id: string;
  providerId: string;
  label: string;
  authKind: "none" | "api_key" | string;
  status: string;
  statusMessage: string | null;
  metadataJson: string;
  createdAt: string;
  updatedAt: string;
  hasSecret: boolean;
}

export interface Project {
  id: string;
  name: string;
  description: string;
  instructions: string;
  archived: boolean;
  createdAt: string;
  updatedAt: string;
  conversationCount: number;
}

export interface Conversation {
  id: string;
  projectId: string | null;
  projectName: string | null;
  title: string;
  pinned: boolean;
  archived: boolean;
  providerId: string | null;
  accountId: string | null;
  model: string | null;
  summary: string | null;
  createdAt: string;
  updatedAt: string;
  lastMessageAt: string | null;
  lastMessage: string | null;
}

export interface Attachment {
  id: string;
  projectId: string | null;
  conversationId: string | null;
  messageId: string | null;
  name: string;
  mime: string;
  size: number;
  path: string;
  sha256: string;
  kind: "image" | "text" | "file" | string;
  textContent: string | null;
  createdAt: string;
}

export interface MessageSegment {
  id: string;
  messageId: string;
  generationId: string | null;
  attemptId: string | null;
  seq: number;
  content: string;
  reasoning: string | null;
  status: string;
  providerId: string | null;
  accountId: string | null;
  model: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface Message {
  id: string;
  conversationId: string;
  seq: number;
  role: "user" | "assistant" | "system" | string;
  content: string;
  reasoning: string | null;
  status: string;
  error: string | null;
  interrupted: boolean;
  interruptionReason: string | null;
  segments: MessageSegment[];
  providerId: string | null;
  accountId: string | null;
  model: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface MessageView {
  message: Message;
  attachments: Attachment[];
}

export interface Task {
  id: string;
  projectId: string | null;
  projectName: string | null;
  conversationId: string | null;
  name: string;
  instructions: string;
  status: string;
  providerId: string | null;
  accountId: string | null;
  model: string | null;
  currentStep: number;
  error: string | null;
  retryCount: number;
  createdAt: string;
  updatedAt: string;
  startedAt: string | null;
  completedAt: string | null;
}

export interface TaskStep {
  id: string;
  taskId: string;
  seq: number;
  title: string;
  instruction: string;
  status: string;
  output: string | null;
  error: string | null;
  startedAt: string | null;
  completedAt: string | null;
}

export interface Checkpoint {
  id: string;
  taskId: string;
  seq: number;
  stateJson: string;
  createdAt: string;
}

export interface Artifact {
  id: string;
  taskId: string | null;
  projectId: string | null;
  name: string;
  path: string;
  mime: string;
  size: number;
  sha256: string;
  createdAt: string;
}

export interface ExecutionLog {
  id: string;
  taskId: string;
  level: string;
  message: string;
  dataJson: string | null;
  createdAt: string;
}

export interface TaskDetail {
  task: Task;
  steps: TaskStep[];
  checkpoints: Checkpoint[];
  logs: ExecutionLog[];
  artifacts: Artifact[];
}

export interface SearchHit {
  conversationId: string;
  conversationTitle: string;
  messageId: string;
  role: string;
  snippet: string;
  createdAt: string;
}

export interface UsageSummary {
  providerId: string | null;
  model: string | null;
  requests: number;
  inputTokens: number;
  outputTokens: number;
}

export interface ProviderHealth {
  ok: boolean;
  message: string;
  latencyMs: number;
  version: string | null;
}

export interface AccountTest {
  ok: boolean;
  message: string;
  latencyMs: number;
  modelCount: number;
}

export interface ModelInfo {
  id: string;
  name: string;
  sizeBytes: number | null;
  family: string | null;
  parameterSize: string | null;
  quantization: string | null;
  capabilities: string[];
  modifiedAt: string | null;
}

export interface BackupInfo {
  path: string;
  sizeBytes: number;
  createdAt: string;
}

export interface AppError {
  code: string;
  message: string;
}

export type StreamEvent =
  | { type: "started"; messageId: string; userMessageId: string }
  | {
      type: "attemptStarted";
      generationId: string;
      attemptNo: number;
      providerId: string;
      accountId: string;
      model: string;
    }
  | { type: "segmentStarted"; messageId: string; segmentId: string; seq: number }
  | {
      type: "retryScheduled";
      attemptNo: number;
      delayMs: number;
      code: string;
      message: string;
    }
  | { type: "interrupted"; messageId: string; reason: string; canContinue: boolean }
  | { type: "delta"; content: string }
  | { type: "reasoning"; content: string }
  | { type: "usage"; inputTokens: number | null; outputTokens: number | null }
  | { type: "done"; finishReason: string | null }
  | { type: "error"; code: string; message: string };

export interface AccountTestResult extends AccountTest {}
