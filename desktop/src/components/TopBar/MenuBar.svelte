<svelte:options runes={true} />

<script lang="ts">
  import { click_outside, elevation } from "🍎/actions";
  import { menubar_menus, setActiveMenu, clearActiveMenu, type MenuSection } from "🍎/state/menubar.svelte";
  import { activeWindow, toggleLaunchpad, toggleTheme, desktop, selectLauncherSection, openStaticApp, openGenericBrowserWindow, lockScreen } from "🍎/state/desktop.svelte";
  import Menu from "./Menu.svelte";

  const title = $derived(activeWindow()?.title ?? "AI Launcher");

  const apple_menu: MenuSection = {
    title: "",
    menu: {
      "about-this-mac": { title: "About This Mac", action: () => openStaticApp("settings") },
      "system-preferences": { title: "System Settings...", breakAfter: true, action: () => openStaticApp("settings") },
      "app-store": { title: "App Store...", action: () => openStaticApp("app-store"), breakAfter: true },
      "force-quit": { title: "Force Quit...", disabled: true, breakAfter: true },
      "sleep": { title: "Sleep", action: () => lockScreen() },
      "restart": { title: "Restart...", action: () => window.location.reload() },
      "shut-down": { title: "Shut Down...", breakAfter: true, action: async () => {
        try {
          const { getCurrentWindow } = await import("@tauri-apps/api/window");
          await getCurrentWindow().close();
        } catch (e) {
          window.close();
        }
      } },
      "lock-screen": { title: "Lock Screen", disabled: true },
      "log-out": { title: "Log Out User...", disabled: true },
    },
  };

  const app_menus: Record<string, MenuSection> = {
    file: {
      title: "File",
      menu: {
        "new-window": { title: "New Window", disabled: true },
        "close-window": { title: "Close Window", disabled: true, breakAfter: true },
        "open": { title: "Open...", disabled: true },
        "save": { title: "Save", disabled: true },
      },
    },
    edit: {
      title: "Edit",
      menu: {
        "undo": { title: "Undo", disabled: true },
        "redo": { title: "Redo", disabled: true, breakAfter: true },
        "cut": { title: "Cut", disabled: true },
        "copy": { title: "Copy", disabled: true },
        "paste": { title: "Paste", disabled: true },
        "select-all": { title: "Select All", disabled: true },
      },
    },
    view: {
      title: "View",
      menu: {
        "overview": { title: "Dashboard", action: () => selectLauncherSection("overview") },
        "catalog": { title: "App Catalog", action: () => selectLauncherSection("catalog") },
        "installed": { title: "Installed Apps", action: () => selectLauncherSection("installed") },
        "running": { title: "Running Apps", action: () => selectLauncherSection("running"), breakAfter: true },
        "fullscreen": { title: "Enter Full Screen", disabled: true },
      },
    },
    window: {
      title: "Window",
      menu: {
        "minimize": { title: "Minimize", disabled: true },
        "zoom": { title: "Zoom", disabled: true, breakAfter: true },
        "bring-all": { title: "Bring All to Front", disabled: true },
      },
    },
    help: {
      title: "Help",
      menu: {
        "launcher-help": { title: "AI Launcher Help", disabled: true, breakAfter: true },
        "swagger-ui": { title: "Open API Docs", action: () => openGenericBrowserWindow("http://localhost:8080/swagger-ui/", "Open API Docs") },
      },
    },
  };

  const all_menus: Record<string, MenuSection> = { apple: apple_menu, ...app_menus };
</script>

<div class="h-full flex relative" use:click_outside={clearActiveMenu}>
  {#each Object.entries(all_menus) as [menuID, menuConfig]}
    <div class="relative h-full">
      <div style:height="100%">
        <button
          class="relative z-10 h-full px-2 text-[0.82rem] font-medium tracking-wide whitespace-nowrap rounded transition-colors text-black dark:text-white {menuID === 'apple' ? 'ml-2 mr-1 px-2.5 flex items-center' : ''} {menubar_menus.active === menuID ? 'bg-black/15 dark:bg-white/15' : 'hover:bg-black/10 dark:hover:bg-white/10'}"
          onclick={() => setActiveMenu(menuID)}
          onmouseover={() => menubar_menus.active && setActiveMenu(menuID)}
          onfocus={() => setActiveMenu(menuID)}
        >
          {#if menuID === "apple"}
            <svg class="w-4 h-4" viewBox="0 0 17 20" fill="currentColor">
              <path d="M15.64 14.85c-.37.82-.54 1.18-.96 1.9-.58.99-1.4 2.22-2.42 2.23-.91.01-1.14-.6-2.38-.59-1.23.01-1.49.6-2.4.6-1.02-.01-1.8-1.13-2.38-2.11C3.16 13.55 2.94 9.88 4.6 7.94c1.17-1.37 3.02-2.17 4.11-2.17 1.53 0 2.47 1.02 3.72 1.02 1.22 0 1.96-1.03 3.72-1.02.9 0 2.53.37 3.49 1.77-3.08 1.68-2.58 6.07 0.53 7.23l-.53.08zM12.14 0c.17 1.22-.35 2.42-1.05 3.26-.73.87-1.96 1.54-3.15 1.5-.2-1.16.38-2.37 1.09-3.17C9.77.71 11.01.08 12.14 0z"/>
            </svg>
          {:else if menuID === Object.keys(app_menus)[0]}
            <strong class="font-bold">{title}</strong>
          {:else}
            {menuConfig.title}
          {/if}
        </button>
      </div>

      <div
        class="absolute mt-[1.5px] z-[9999]"
        style:visibility={menubar_menus.active === menuID ? "visible" : "hidden"}
        use:elevation={"menubar-menu-parent"}
      >
        <Menu menu={menuConfig.menu} />
      </div>
    </div>
  {/each}
</div>
