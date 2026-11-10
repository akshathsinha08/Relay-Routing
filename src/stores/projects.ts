import { create } from "zustand";

import { api, errorMessage } from "@/lib/api";
import type { Artifact, Attachment, Project } from "@/types";

import { useUiStore } from "./ui";

interface ProjectsState {
  projects: Project[];
  active: Project | null;
  attachments: Attachment[];
  artifacts: Artifact[];
  loading: boolean;
  load: () => Promise<void>;
  open: (id: string) => Promise<void>;
  create: (
    name: string,
    description?: string,
    instructions?: string,
  ) => Promise<Project | null>;
  update: (
    id: string,
    patch: {
      name?: string;
      description?: string;
      instructions?: string;
      archived?: boolean;
    },
  ) => Promise<boolean>;
  remove: (id: string) => Promise<void>;
  refreshDetail: () => Promise<void>;
  importFiles: (paths: string[]) => Promise<void>;
  deleteAttachment: (id: string) => Promise<void>;
}

export const useProjectsStore = create<ProjectsState>()((set, get) => ({
  projects: [],
  active: null,
  attachments: [],
  artifacts: [],
  loading: false,
  async load() {
    set({ loading: true });
    try {
      const projects = await api.listProjects(true);
      set({ projects, loading: false });
    } catch (error) {
      set({ loading: false });
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async open(id) {
    try {
      const [active, attachments, artifacts] = await Promise.all([
        api.getProject(id),
        api.listProjectAttachments(id),
        api.listProjectArtifacts(id),
      ]);
      set({ active, attachments, artifacts });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async create(name, description = "", instructions = "") {
    try {
      const project = await api.createProject(name, description, instructions);
      await get().load();
      return project;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
  async update(id, patch) {
    try {
      const project = await api.updateProject(id, patch);
      set((state) => ({
        projects: state.projects.map((item) => (item.id === id ? project : item)),
        active: state.active?.id === id ? project : state.active,
      }));
      return true;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async remove(id) {
    try {
      await api.deleteProject(id);
      set((state) => ({
        projects: state.projects.filter((item) => item.id !== id),
        active: state.active?.id === id ? null : state.active,
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async refreshDetail() {
    const active = get().active;
    if (!active) return;
    try {
      const [project, attachments, artifacts] = await Promise.all([
        api.getProject(active.id),
        api.listProjectAttachments(active.id),
        api.listProjectArtifacts(active.id),
      ]);
      set({ active: project, attachments, artifacts });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async importFiles(paths) {
    const active = get().active;
    if (!active || paths.length === 0) return;
    try {
      await api.importAttachments({ projectId: active.id, paths });
      await get().refreshDetail();
      useUiStore.getState().toast(`Attached ${paths.length} file(s) to project`, "success");
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async deleteAttachment(id) {
    try {
      await api.deleteAttachment(id);
      set((state) => ({
        attachments: state.attachments.filter((item) => item.id !== id),
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
}));
