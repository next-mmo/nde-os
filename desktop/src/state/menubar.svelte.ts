export type MenuItem = {
  title: string;
  disabled?: boolean;
  breakAfter?: boolean;
  action?: () => void;
};

export type MenuSection = {
  title: string;
  menu: Record<string, MenuItem>;
};

export const menubar_menus = $state({
  active: "" as string,
});

export function setActiveMenu(id: string) {
  menubar_menus.active = id;
}

export function clearActiveMenu() {
  menubar_menus.active = "";
}
