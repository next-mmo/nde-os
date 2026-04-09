export function engineIcon(engine: string) { return engine === "wayfern" ? "🌐" : "🦊"; }
export function engineLabel(engine: string) { return engine === "wayfern" ? "Wayfern (Chromium)" : "Camoufox (Firefox)"; }
export function formatDate(epoch: number) { return new Date(epoch * 1000).toLocaleDateString(); }
export function formatDateTime(epoch: number) {
  const d = new Date(epoch * 1000);
  return d.toLocaleDateString() + " " + d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
