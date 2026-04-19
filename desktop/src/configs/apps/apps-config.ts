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
  chat: create_app_config({
    title: "NDE Chat",
    width: 900,
    height: 680,
  }),
  "app-store": create_app_config({
    title: "App Store",
    width: 860,
    height: 620,
  }),
  terminal: create_app_config({
    title: "Terminal",
    width: 600,
    height: 400,
  }),
  "code-editor": create_app_config({
    title: "Code Editor",
    width: 1200,
    height: 800,
  }),
  "command-center": create_app_config({
    title: "Command Center",
    width: 980,
    height: 720,
    dock_breaks_before: true,
  }),
  "model-settings": create_app_config({
    title: "LLM Providers",
    width: 880,
    height: 640,
  }),
  plugins: create_app_config({
    title: "Plugins",
    width: 920,
    height: 660,
  }),
  channels: create_app_config({
    title: "Channels",
    width: 880,
    height: 640,
  }),
  "mcp-tools": create_app_config({
    title: "MCP Tools",
    width: 920,
    height: 660,
  }),
  skills: create_app_config({
    title: "Skills",
    width: 920,
    height: 660,
  }),
  knowledge: create_app_config({
    title: "Knowledge",
    width: 960,
    height: 680,
  }),
  architecture: create_app_config({
    title: "Architecture",
    width: 1200,
    height: 800,
  }),
  "shield-browser": create_app_config({
    title: "Shield Browser",
    width: 1100,
    height: 760,
    dock_breaks_before: true,
  }),
  "file-explorer": create_app_config({
    title: "File Explorer",
    width: 1100,
    height: 760,
    dock_breaks_before: true,
  }),
  "vibe-studio": create_app_config({
    title: "Vibe Code Studio",
    width: 1280,
    height: 800,
  }),
  screenshot: create_app_config({
    title: "Screenshot Results",
    width: 800,
    height: 600,
  }),
  "service-hub": create_app_config({
    title: "Service Hub",
    width: 780,
    height: 640,
  }),
  "freecut": create_app_config({
    title: "FreeCut",
    width: 1400,
    height: 860,
    dock_breaks_before: true,
  }),
  launchpad: create_app_config({
    title: "Launchpad",
    should_open_window: false,
    dock_breaks_before: true,
  }),
  "download-center": create_app_config({
    title: "Download Center",
    width: 1000,
    height: 720,
    dock_breaks_before: true,
  }),
  "video-player": create_app_config({
    title: "Video Player",
    width: 1120,
    height: 720,
  }),
};

export type StaticAppID = keyof typeof apps_config;
