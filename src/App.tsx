import { useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { loadSettingsFormData } from "./lib/settings";
import { AppRouter } from "./router";
import { useAppStore } from "./store/appStore";
import "./styles.css";

const queryClient = new QueryClient();

function SettingsBootstrap() {
  const setAccountMode = useAppStore((state) => state.setAccountMode);

  useEffect(() => {
    let cancelled = false;

    loadSettingsFormData()
      .then((settings) => {
        if (!cancelled) {
          setAccountMode(settings.accountMode);
        }
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
    };
  }, [setAccountMode]);

  return null;
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <SettingsBootstrap />
      <AppRouter />
    </QueryClientProvider>
  );
}

export default App;
