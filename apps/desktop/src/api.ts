import { invoke } from "@tauri-apps/api/core";

// Tipos espejo de `crates/drivward-ipc`.
export type Mode = "observe" | "recommend" | "automatic";
export type Health = "stable" | "attention" | "unstable" | "paused";
export type Category = "network" | "storage" | "display" | "chipset" | "audio" | "other";
export type DeviceState = "missing_driver" | "error" | "updatable" | "ok" | "unknown";
export type Origin = "drivward" | "external" | "rollback";
export type Confidence = "none" | "low" | "medium" | "high";

export interface Settings {
  mode: Mode;
  quarantine_days: number;
  observation_days: number;
  check_hour: number;
  paused_until: string | null;
  auto_rollback: boolean;
}

export interface Status {
  health: Health;
  message: string;
  mode: Mode;
  paused_until: string | null;
  last_scan: string | null;
  devices_total: number;
  devices_attention: number;
  recommendations: number;
  events_last_7_days: number;
  changes_in_observation: number;
  version: string;
}

export interface DeviceView {
  instance_id: string;
  name: string;
  category: Category;
  state: DeviceState;
  provider: string | null;
  version: string | null;
  date: string | null;
}

export interface ChangeView {
  id: number;
  at: string;
  device_id: string;
  device_name: string;
  category: Category;
  old_version: string | null;
  new_version: string | null;
  origin: Origin;
  validated: boolean;
  can_rollback: boolean;
}

export interface EvidenceView {
  change_id: number;
  device_name: string;
  events_before: number;
  events_after: number;
  days_before: number;
  days_after: number;
  confidence: Confidence;
  explanation: string;
  suggest_rollback: boolean;
}

export interface RecommendationView {
  id: string;
  device_id: string;
  device_name: string;
  category: Category;
  current_version: string | null;
  new_version: string | null;
  source: string;
  reasons: string[];
  requires_approval: boolean;
  available_after: string | null;
  blocked_reason: string | null;
}

export interface BlockView {
  update_id: string;
  title: string;
  hardware_id: string | null;
  created_at: string;
}

export type Request =
  | { type: "status" }
  | { type: "devices" }
  | { type: "history"; limit: number }
  | { type: "recommendations" }
  | { type: "evidence" }
  | { type: "blocks" }
  | { type: "settings" }
  | { type: "set_settings"; settings: Settings }
  | { type: "pause"; hours: number }
  | { type: "resume" }
  | { type: "scan_now" }
  | { type: "install"; recommendation_id: string }
  | { type: "rollback"; change_id: number }
  | { type: "mark_good"; change_id: number }
  | { type: "block"; update_id: string }
  | { type: "unblock"; update_id: string };

type Response =
  | { type: "status"; data: Status }
  | { type: "devices"; data: DeviceView[] }
  | { type: "history"; data: ChangeView[] }
  | { type: "recommendations"; data: RecommendationView[] }
  | { type: "evidence"; data: EvidenceView[] }
  | { type: "blocks"; data: BlockView[] }
  | { type: "settings"; data: Settings }
  | { type: "done"; data: { message: string } }
  | { type: "error"; data: { message: string } };

async function call(request: Request): Promise<Response> {
  const response = await invoke<Response>("service_request", { request });
  if (response.type === "error") throw new Error(response.data.message);
  return response;
}

async function data<T>(request: Request): Promise<T> {
  const response = await call(request);
  return (response as unknown as { data: T }).data;
}

export const api = {
  status: () => data<Status>({ type: "status" }),
  devices: () => data<DeviceView[]>({ type: "devices" }),
  history: (limit = 100) => data<ChangeView[]>({ type: "history", limit }),
  recommendations: () => data<RecommendationView[]>({ type: "recommendations" }),
  evidence: () => data<EvidenceView[]>({ type: "evidence" }),
  blocks: () => data<BlockView[]>({ type: "blocks" }),
  settings: () => data<Settings>({ type: "settings" }),
  /** Acciones: devuelven el mensaje del servicio. */
  act: async (request: Request) => (await data<{ message: string }>(request)).message,
  canControl: () => invoke<boolean>("can_control"),
  relaunchElevated: () => invoke<void>("relaunch_elevated"),
};

export const CATEGORY: Record<Category, string> = {
  network: "Red",
  storage: "Almacenamiento",
  display: "Gráfica",
  chipset: "Chipset",
  audio: "Audio",
  other: "Otro",
};

export const STATE: Record<DeviceState, string> = {
  missing_driver: "Falta driver",
  error: "Con error",
  updatable: "Actualizable",
  ok: "Correcto",
  unknown: "Desconocido",
};

export const CONFIDENCE: Record<Confidence, string> = {
  none: "Sin evidencia",
  low: "Baja",
  medium: "Media",
  high: "Alta",
};

export const MODE: Record<Mode, { label: string; text: string }> = {
  observe: { label: "Observar", text: "Solo registra cambios y fallos. No instala nada." },
  recommend: { label: "Recomendar", text: "Propone cambios con su explicación y espera tu aprobación." },
  automatic: {
    label: "Automático",
    text: "Instala solo drivers de bajo riesgo. Gráfica, chipset, almacenamiento y red siempre piden aprobación.",
  },
};

export function formatDate(value: string | null): string {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("es", { dateStyle: "medium", timeStyle: value.includes("T") ? "short" : undefined });
}
