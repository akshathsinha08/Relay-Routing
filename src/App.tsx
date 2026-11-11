import { Loader2 } from "lucide-react";
import { useEffect } from "react";

import { Sidebar } from "@/components/Sidebar";
import { Toasts } from "@/components/Toasts";
import { TopBar } from "@/components/TopBar";
import { useAppStore } from "@/stores/app";
import { useChatStore } from "@/stores/chat";
import { useUiStore } from "@/stores/ui";
import { ChatView } from "@/views/ChatView";
import { Onboarding } from "@/views/Onboarding";
import { ProjectDetail } from "@/views/ProjectDetail";
import { ProjectsView } from "@/views/ProjectsView";
import { ProvidersView } from "@/views/ProvidersView";
import { SettingsView } from "@/views/SettingsView";
import { TasksView } from "@/views/TasksView";

export default function App() {
  const ready = useAppStore((state) => state.ready);
  const loading = useAppStore((state) => state.loading);
  const init = useAppStore((state) => state.init);
  const view = useUiStore((state) => state.view);
  const navigate = useUiStore((state) => state.navigate);
  const newConversation = useChatStore((state) => state.newConversation);

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key === "n") {
        event.preventDefault();
        void newConversation();
      } else if (event.key === ",") {
        event.preventDefault();
        navigate({ name: "settings" });
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate, newConversation]);

  if (!ready) {
    return (
      <div className="empty" style={{ height: "100vh" }}>
        {loading ? <Loader2 size={22} className="spin" /> : null}
        <h3>Relay Routing</h3>
        <p>{loading ? "Opening local workspace…" : "Loading…"}</p>
      </div>
    );
  }

  return (
    <>
      <div className="app-shell" aria-hidden={view.name === "onboarding"}>
        <Sidebar />
        <div className="main">
          <TopBar />
          {view.name === "chat" ? <ChatView /> : null}
          {view.name === "projects" ? <ProjectsView /> : null}
          {view.name === "project" ? <ProjectDetail id={view.id} /> : null}
          {view.name === "tasks" ? <TasksView /> : null}
          {view.name === "providers" ? <ProvidersView /> : null}
          {view.name === "settings" ? <SettingsView /> : null}
        </div>
      </div>
      {view.name === "onboarding" ? <Onboarding /> : null}
      <Toasts />
    </>
  );
}
