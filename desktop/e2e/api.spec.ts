import { test, expect } from "./fixtures";

const API = "http://127.0.0.1:8080";

test.describe("Core API Integrations", () => {
  
  test.describe("Tauri Native IPC Commands", () => {
    test("get_system_info should return hardware OS info", async ({ page }) => {
      const info = await page.evaluate(async () => {
        return (window as any).__TAURI_INTERNALS__.invoke('get_system_info');
      });
      expect(info).toBeDefined();
      expect(info.os).toBeTruthy();
      expect(info.cpu).toBeTruthy();
    });

    test("get_resource_usage should return active memory and cpu", async ({ page }) => {
      const usage = await page.evaluate(async () => {
        return (window as any).__TAURI_INTERNALS__.invoke('get_resource_usage');
      });
      expect(usage).toBeDefined();
      expect(typeof usage.cpu_usage_percent).toBe('number');
      expect(typeof usage.memory_used_mb).toBe('number');
    });

    test("health_check should return ok", async ({ page }) => {
      const health = await page.evaluate(async () => {
        return (window as any).__TAURI_INTERNALS__.invoke('health_check');
      });
      expect(health).toContain('ok');
    });

    test("list_apps should return an array", async ({ page }) => {
      const apps = await page.evaluate(async () => {
        return (window as any).__TAURI_INTERNALS__.invoke('list_apps');
      });
      expect(Array.isArray(apps)).toBe(true);
    });

    test("get_catalog should return catalog manifest", async ({ page }) => {
      const catalog = await page.evaluate(async () => {
        return (window as any).__TAURI_INTERNALS__.invoke('get_catalog');
      });
      expect(Array.isArray(catalog)).toBe(true);
    });
  });

  test.describe("HTTP Fallback APIs", () => {
    test("GET /api/models should list models", async ({ page }) => {
      const response = await page.request.get(`${API}/api/models`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
      expect(Array.isArray(json.data)).toBe(true);
    });

    test("GET /api/agent/config should return agent settings", async ({ page }) => {
      const response = await page.request.get(`${API}/api/agent/config`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });

    test("GET /api/plugins should list plugins", async ({ page }) => {
      const response = await page.request.get(`${API}/api/plugins`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });

    test("GET /api/skills should return skills array", async ({ page }) => {
      const response = await page.request.get(`${API}/api/skills`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });

    test("GET /api/mcp/tools should retrieve MCP tool definitions", async ({ page }) => {
      const response = await page.request.get(`${API}/api/mcp/tools`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });

    test("GET /api/mcp/servers should retrieve MCP server definitions", async ({ page }) => {
      const response = await page.request.get(`${API}/api/mcp/servers`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });

    test("GET /api/knowledge should fetch knowledge database items", async ({ page }) => {
      const response = await page.request.get(`${API}/api/knowledge`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });

    test("GET /api/memory should pull persistent config", async ({ page }) => {
      const response = await page.request.get(`${API}/api/memory`);
      expect(response.ok()).toBe(true);
      const json = await response.json();
      expect(json.success).toBe(true);
    });
  });
});
