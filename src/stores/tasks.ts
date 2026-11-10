import { create } from "zustand";

import { api, errorMessage } from "@/lib/api";
import type { Task, TaskDetail } from "@/types";

import { isTaskActive } from "@/lib/format";

import { useUiStore } from "./ui";

interface CreateTaskInput {
  name: string;
  instructions: string;
  projectId?: string | null;
  conversationId?: string | null;
  providerId?: string | null;
  accountId?: string | null;
  model?: string | null;
}

interface TasksState {
  tasks: Task[];
  detail: TaskDetail | null;
  loading: boolean;
  busy: boolean;
  load: (projectId?: string) => Promise<void>;
  open: (taskId: string) => Promise<void>;
  create: (payload: CreateTaskInput) => Promise<Task | null>;
  start: (taskId: string) => Promise<boolean>;
  pause: (taskId: string) => Promise<boolean>;
  cancel: (taskId: string) => Promise<boolean>;
  resume: (taskId: string) => Promise<boolean>;
  remove: (taskId: string) => Promise<boolean>;
  openArtifact: (id: string) => Promise<void>;
  stopPolling: () => void;
}

let pollTimer: ReturnType<typeof setTimeout> | null = null;
let pollToken = 0;

export const useTasksStore = create<TasksState>()((set, get) => ({
  tasks: [],
  detail: null,
  loading: false,
  busy: false,
  async load(projectId) {
    set({ loading: true });
    try {
      const tasks = await api.listTasks(projectId);
      set({ tasks, loading: false });
    } catch (error) {
      set({ loading: false });
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  stopPolling() {
    pollToken += 1;
    if (pollTimer) {
      clearTimeout(pollTimer);
      pollTimer = null;
    }
  },
  async open(taskId) {
    get().stopPolling();
    const token = pollToken;
    try {
      const detail = await api.getTaskDetail(taskId);
      set({ detail });
      if (isTaskActive(detail.task)) {
        pollTimer = setTimeout(() => {
          if (pollToken !== token) return;
          void get().open(taskId);
        }, 1200);
      }
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async create(payload) {
    try {
      const task = await api.createTask(payload);
      await get().load(payload.projectId ?? undefined);
      await get().open(task.id);
      return task;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
  async start(taskId) {
    set({ busy: true });
    try {
      const task = await api.startTask(taskId);
      set((state) => ({
        busy: false,
        tasks: upsertTask(state.tasks, task),
      }));
      await get().open(taskId);
      useUiStore.getState().toast("Task started.", "success");
      return true;
    } catch (error) {
      set({ busy: false });
      useUiStore.getState().toast(errorMessage(error), "error");
      await get().open(taskId);
      return false;
    }
  },
  async pause(taskId) {
    set({ busy: true });
    try {
      const task = await api.pauseTask(taskId);
      set((state) => ({ busy: false, tasks: upsertTask(state.tasks, task) }));
      await get().open(taskId);
      return true;
    } catch (error) {
      set({ busy: false });
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async cancel(taskId) {
    set({ busy: true });
    try {
      const task = await api.cancelTask(taskId);
      set((state) => ({ busy: false, tasks: upsertTask(state.tasks, task) }));
      await get().open(taskId);
      return true;
    } catch (error) {
      set({ busy: false });
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async resume(taskId) {
    set({ busy: true });
    try {
      const task = await api.resumeTask(taskId);
      set((state) => ({ busy: false, tasks: upsertTask(state.tasks, task) }));
      await get().open(taskId);
      return true;
    } catch (error) {
      set({ busy: false });
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async remove(taskId) {
    get().stopPolling();
    try {
      await api.deleteTask(taskId);
      set((state) => ({
        tasks: state.tasks.filter((item) => item.id !== taskId),
        detail: state.detail?.task.id === taskId ? null : state.detail,
      }));
      return true;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async openArtifact(id) {
    try {
      await api.openArtifact(id);
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
}));

function upsertTask(tasks: Task[], task: Task): Task[] {
  const exists = tasks.some((item) => item.id === task.id);
  if (!exists) return [task, ...tasks];
  return tasks.map((item) => (item.id === task.id ? task : item));
}
