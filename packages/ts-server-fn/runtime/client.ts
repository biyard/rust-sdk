// Hand-written runtime for the ts-server-fn generated client.
//
// This file is the ONLY place that knows about transport. The generated
// handler functions (one per Rust #[get]/#[post]/... handler) are thin typed
// wrappers that build a URL + body and delegate here. Copy this file into
// your TS package at `src/runtime/client.ts` — ts-server-fn does NOT
// generate it.
//
// Wire contract (must stay in lockstep with the dioxus-fullstack server and
// the asset `common/fullstack/server_fn.rs` Rust client):
//   - URL: path template with `{}` substituted (path args urlencoded) plus
//     `?k=v` query (None/undefined skipped). Built by the generated fn.
//   - Method: per the Rust attribute (#[get]/#[post]/#[put]/#[patch]/#[delete]).
//   - Body (POST/PUT/PATCH): a JSON object keyed by the body arg name, i.e.
//     `{ "<argName>": <value> }`. The generated fn passes this object already
//     shaped; this runtime just JSON-stringifies it.
//   - Success (2xx): the response body is the plain JSON of the return value.
//   - Failure (non-2xx): throw `ApiError` carrying the status + body text.
//   - Auth: same-origin cookies via `credentials: "include"`.

/** Base URL prepended to every request path. Empty = same origin. */
let BASE_URL = "";

/** Override the API base URL (e.g. for a native/mobile build hitting a remote host). */
export function setBaseUrl(url: string): void {
  BASE_URL = url.replace(/\/+$/, "");
}

/** Optional header injector — e.g. a bearer token for mobile builds. */
let HEADER_PROVIDER: (() => Record<string, string>) | null = null;

/** Register a function that returns extra headers attached to every request. */
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

type BodyObject = Record<string, unknown>;

async function request<T>(
  method: string,
  path: string,
  body?: BodyObject,
): Promise<T> {
  const url = `${BASE_URL}${path}`;

  const headers: Record<string, string> = {};
  if (HEADER_PROVIDER) Object.assign(headers, HEADER_PROVIDER());

  const init: RequestInit = {
    method,
    credentials: "include",
    headers,
  };

  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body);
  }

  const res = await fetch(url, init);

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new ApiError(res.status, text);
  }

  // 204 / empty body → undefined as T (void handlers).
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  if (text.length === 0) return undefined as T;
  return JSON.parse(text) as T;
}

export function apiGet<T>(path: string): Promise<T> {
  return request<T>("GET", path);
}

export function apiPost<T>(path: string, body: BodyObject): Promise<T> {
  return request<T>("POST", path, body);
}

export function apiPut<T>(path: string, body: BodyObject): Promise<T> {
  return request<T>("PUT", path, body);
}

export function apiPatch<T>(path: string, body: BodyObject): Promise<T> {
  return request<T>("PATCH", path, body);
}

export function apiDelete<T>(path: string): Promise<T> {
  return request<T>("DELETE", path);
}
