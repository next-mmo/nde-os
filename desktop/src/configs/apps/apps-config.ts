import { create_app_config } from "🍎/helpers/create-app-config";

export const apps_config = {
  "ai-launcher": create_app_config({
    title: "AI Launcher",
    width: 1240,
    height: 760,
  }),
  browser: create_app_config({
    title: "Browser",
    width: 1040,
    height: 720,
    dock_breaks_before: true,
  }),
  logs: create_app_config({
    title: "Logs",
    width: 860,
    height: 620,
  }),
  settings: create_app_config({
    title: "Settings",
    width: 820,
    height: 620,
  }),
  "app-store": create_app_config({
    title: "App Store",
    width: 860,
    height: 620,
  }),
  terminal: create_app_config({
    title: "Terminal",
    width: 840,
    height: 540,
  }),
  launchpad: create_app_config({
    title: "Launchpad",
    should_open_window: false,
    dock_breaks_before: true,
  }),
};

export type StaticAppID = keyof typeof apps_config;
