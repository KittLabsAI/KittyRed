import { create } from "zustand";
import type { AccountMode, SettingsTabId } from "../lib/types";

type AppStore = {
  assistantOpen: boolean;
  accountMode: AccountMode;
  activeSettingsTab: SettingsTabId;
  openAssistant: () => void;
  closeAssistant: () => void;
  setAccountMode: (mode: AccountMode) => void;
  setActiveSettingsTab: (tab: SettingsTabId) => void;
};

export const useAppStore = create<AppStore>((set) => ({
  assistantOpen: false,
  accountMode: "paper",
  activeSettingsTab: "exchanges",
  openAssistant: () => set({ assistantOpen: true }),
  closeAssistant: () => set({ assistantOpen: false }),
  setAccountMode: (mode) => set({ accountMode: mode }),
  setActiveSettingsTab: (tab) => set({ activeSettingsTab: tab }),
}));
