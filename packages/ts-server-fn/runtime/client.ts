// Hand-written runtime for the ts-server-fn generated client.
//
// This file is the ONLY place that knows about transport. The generated
// handler functions (one per Rust #[get]/#[post]/... handler) are thin typed
// wrappers that build a URL and forward the body/query objects here. Copy this
// file into your TS package at `src/runtime/client.ts` — it is NOT generated.
//
// Wire contract (D2 — must stay in lockstep with the axum server; see the
// round-trip note at the bottom):
//   - URL: path template with path args interpolated (urlencoded) by the
//     generated fn. Query is appended here from the `Query<T>` object.
//   - Method: per the Rust attribute.
//   - Body (POST/PUT/PATCH): the DTO JSON **directly** — NOT `{ "<arg>": ... }`.
//   - Success (2xx): response body is the plain JSON of the return value;
//     204 / empty body → `undefined` (void handlers).
//   - Failure (non-2xx): throw `ApiError` carrying the status + body text.
//   - Auth: same-origin cookies via `credentials: "include"`.

/** Base URL prepended to every request path. Empty = same origin. */
let BASE_URL = "";

/** Override the API base URL (e.g. a native/mobile build hitting a remote host). */
export function setBaseUrl(url: string): void {
  BASE_URL = url.replace(/\/+$/, "");
}

/** Optional header injector — e.g. a bearer token for mobile builds. */
let HEADER_PROVIDER: (() => Record<string, string>) | null = null;

export function setHeaderProvider(fn: (() => Record<string, string>) | null): void {
  HEADER_PROVIDER = fn;
}

/** Thrown on any non-2xx response. */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
  ) {
    super(`HTTP ${status}: ${body}`);
    this.name = "ApiError";
  }
}

/** Serialize a `Query<T>` object to a `?k=v` string (skips null/undefined). */
function toQuery(query?: Record<string, unknown>): string {
  if (!query) return "";
  const sp = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) {
    if (v !== null && v !== undefined) sp.set(k, String(v));
  }
  const s = sp.toString();
  return s ? `?${s}` : "";
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  query?: Record<string, unknown>,
): Promise<T> {
  const url = `${BASE_URL}${path}${toQuery(query)}`;

  const headers: Record<string, string> = {};
  if (HEADER_PROVIDER) Object.assign(headers, HEADER_PROVIDER());

  const init: RequestInit = { method, credentials: "include", headers };

  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body); // D2: DTO sent directly, no wrapping.
  }

  const res = await fetch(url, init);

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new ApiError(res.status, text);
  }

  if (res.status === 204) return undefined as T;
  const text = await res.text();
  if (text.length === 0) return undefined as T;
  return JSON.parse(text) as T;
}

export function apiGet<T>(path: string, query?: Record<string, unknown>): Promise<T> {
  return request<T>("GET", path, undefined, query);
}

export function apiDelete<T>(path: string, query?: Record<string, unknown>): Promise<T> {
  return request<T>("DELETE", path, undefined, query);
}

export function apiPost<T>(path: string, body?: unknown, query?: Record<string, unknown>): Promise<T> {
  return request<T>("POST", path, body, query);
}

export function apiPut<T>(path: string, body?: unknown, query?: Record<string, unknown>): Promise<T> {
  return request<T>("PUT", path, body, query);
}

export function apiPatch<T>(path: string, body?: unknown, query?: Record<string, unknown>): Promise<T> {
  return request<T>("PATCH", path, body, query);
}

// ── Lockstep note (Risk #4) ─────────────────────────────────────────
// The generated fns and this runtime form one wire contract. To prevent
// silent drift, the macro's snapshot tests pin the exact generated call
// (`apiPost<T>(\`/url\`, body)`) and this runtime's parameter order
// (path, body, query) must match those calls. If you change one, change both
// and re-bless the snapshots.
