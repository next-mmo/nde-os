const z_index_config = {
  wallpaper: -1,
  topbar: 90,
  dock: 80,
  launchpad: 100,
  window_traffic_lights: 12,
  "menubar-menu-parent": 200,
  "action-center-panel": 210,
};

if (typeof document !== "undefined") {
  for (const [element, zIndexValue] of Object.entries(z_index_config)) {
    document.body.style.setProperty(`--system-z-index-${element}`, `${zIndexValue}`);
  }
}

export function elevation(node: HTMLElement, uiElement: keyof typeof z_index_config) {
  node.style.zIndex = `var(--system-z-index-${uiElement})`;
}
