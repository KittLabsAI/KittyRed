import { Outlet } from "react-router-dom";
import { AssistantDrawer } from "../assistant/AssistantDrawer";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";
import { useAppStore } from "../../store/appStore";

export function AppShell() {
  const assistantOpen = useAppStore((state) => state.assistantOpen);
  const closeAssistant = useAppStore((state) => state.closeAssistant);

  return (
    <>
      <div className="app-shell min-h-screen bg-background text-foreground">
        <Sidebar />
        <main className="content min-h-screen">
          <Header />
          <Outlet />
        </main>
      </div>
      <AssistantDrawer onClose={closeAssistant} open={assistantOpen} />
    </>
  );
}
