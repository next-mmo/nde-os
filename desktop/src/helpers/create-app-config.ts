export type AppConfig = {
  title: string;
  width?: number;
  height?: number;
  resizable?: boolean;
  expandable?: boolean;
  dock_breaks_before?: boolean;
  should_open_window?: boolean;
};

export const create_app_config = (config: AppConfig) => ({
  width: 960,
  height: 640,
  resizable: true,
  expandable: true,
  dock_breaks_before: false,
  should_open_window: true,
  ...config,
});
