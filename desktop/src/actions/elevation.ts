const z_index_config = {
  wallpaper: -1,
  topbar: 9100,
  dock: 9000,
  launchpad: 9500,
  window_traffic_lights: 12,
  "menubar-menu-parent": 9800,
  "action-center-panel": 9810,
  "system-info-panel": 9820,
};

if (typeof document !== "undefined") {
  for (const [element, zIndexValue] of Object.entries(z_index_config)) {
    document.body.style.setProperty(`--system-z-index-${element}`, `${zIndexValue}`);
  }
}

export function elevation(node: HTMLElement, uiElement: keyof typeof z_index_config) {
  node.style.zIndex = `var(--system-z-index-${uiElement})`;
}
