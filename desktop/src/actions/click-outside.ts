export function click_outside(node: HTMLElement, callback: () => void) {
  const handlePointerDown = (event: PointerEvent) => {
    if (!node.contains(event.target as Node)) {
      callback();
    }
  };

  document.addEventListener("pointerdown", handlePointerDown, true);

  return {
    destroy() {
      document.removeEventListener("pointerdown", handlePointerDown, true);
    },
  };
}
