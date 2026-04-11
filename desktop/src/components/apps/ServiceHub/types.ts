export type ServiceStatus = {
  id: string;
  name: string;
  description: string;
  group: "voice" | "media" | "ai" | "tooling";
  installed: boolean;
  version?: string | null;
  path?: string | null;
  usedBy: string[];
  optional: boolean;
  details?: string | null;
};
