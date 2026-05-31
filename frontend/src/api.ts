const API = import.meta.env.DEV
  ? "" // Dev: Vite proxy handles /v1 → localhost:3000
  : "http://127.0.0.1:33300"; // Production: axum server default bind

export async function fetchJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API}${path}`);
  if (!res.ok) throw new Error(`fetch ${path}: ${res.status}`);
  return res.json() as Promise<T>;
}

export async function putJson<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    method: "PUT",
    headers: {"content-type": "application/json"},
    body: JSON.stringify(body),
  });
  return res.json() as Promise<T>;
}

export async function postJson<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    method: "POST",
    headers: {"content-type": "application/json"},
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.detail ?? `POST ${path}: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

// ── Types ──

export interface VersionInfo {
  app_name: string;
  version: string;
  config_source: string | null;
  bind: string;
  active_provider?: string;
}

export interface ProviderStatus {
  name: string;
  base_url: string;
  model: string;
  models: string[];
  thinking: boolean;
  has_key: boolean;
  is_active: boolean;
  active_model: string;
  default_temperature?: number | null;
  default_top_p?: number | null;
  default_max_output_tokens?: number | null;
}

export interface ProvidersResponse {
  active_provider: string;
  active_model: string;
  key_hash: string;
  providers: ProviderStatus[];
}

export interface ModelStatsEntry {
  provider_model: string;
  model_route: string;
  request_count: number;
  error_count: number;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cached_tokens: number;
  reasoning_tokens: number;
  total_provider_ms: number;
  total_total_ms: number;
  last_status: number;
}

export interface ModelStatsSummary {
  model_count: number;
  total_requests: number;
  total_tokens: number;
  total_prompt_tokens: number;
  total_completion_tokens: number;
  total_cached_tokens: number;
  total_reasoning_tokens: number;
  total_provider_ms: number;
  total_total_ms: number;
}

export interface ModelStatsResponse {
  summary: ModelStatsSummary;
  models: ModelStatsEntry[];
}

export interface ConfigContent {
  content: string;
  path?: string;
  checksum?: string;
}

export interface TrcRecord {
  seq: number;
  trace_id: string;
  created_at: string;
  client_model: string;
  provider_model: string;
  model_route: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cached_tokens: number;
  reasoning_tokens: number;
  provider_ms: number;
  request_time_ms: number;
  status_code: number;
  error: string;
}

export interface TrcResponse {
  records: TrcRecord[];
}

export interface GenerationParams {
  temperature: number | null;
  top_p: number | null;
}