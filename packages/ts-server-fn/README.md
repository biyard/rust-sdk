# ts-server-fn

`#[get]/#[post]/#[put]/#[patch]/#[delete]` attribute macros that turn an axum
handler into **(1) a pure axum route** (no Dioxus) and **(2) a type-safe
TypeScript client function** — from the *signature itself*, with no parallel
annotation to drift.

```rust
use axum::extract::{Path, Json, Query};
use ts_server_fn::{get, post};

#[get("/api/posts/{id}")]
pub async fn get_post(Path(id): Path<String>, user: User) -> ApiResult<Post> { ... }

#[post("/api/posts")]
pub async fn create_post(Json(req): Json<CreatePost>) -> ApiResult<Post> { ... }
```

expands to (illustrative):

```rust
async fn __ts_impl_get_post(Path(id): Path<String>, user: User) -> ApiResult<Post> { ... }

pub async fn get_post(__a0: Path<String>, __a1: User) -> axum::response::Response {
    match __ts_impl_get_post(__a0, __a1).await {
        Ok(v)  => (StatusCode::OK, Json(v)).into_response(),
        Err(e) => (e.as_status_code(), Json(e)).into_response(),
    }
}
ts_server_fn_axum::inventory::submit! { ApiRoute { method: "GET", path: "/api/posts/{id}",
    register: |r| r.route("/api/posts/{id}", axum::routing::get(get_post)) } }
```

and, when `TS_SERVER_FN_PACKAGE_DIR` is set at build time, writes:

```typescript
// $DIR/src/handlers/getPost.ts
import type { Post } from "../types/Post";
import { apiGet } from "../runtime/client";
export async function getPost(id: string): Promise<Post> {
  return apiGet<Post>(`/api/posts/${encodeURIComponent(String(id))}`);
}
```

## Runtime

Two pieces the consumer wires up:

- **`ts-server-fn-axum`** (sibling crate) — `ApiRoute`, `api_router()`
  (collects every handler via `inventory`), and the `AsStatusCode` trait your
  error enums implement.
- **`runtime/client.ts`** — copy into your TS package at `src/runtime/client.ts`.
  Owns transport (base URL, cookies, status→throw, `204`→void, querystring).

```rust
let app = ts_server_fn_axum::api_router();          // every #[get]/#[post]/... handler
```

## Classification (signature-type-based)

| Signature arg            | Becomes        | TS                                   |
|--------------------------|----------------|--------------------------------------|
| `Path(x): Path<T>`       | path arg       | interpolated, urlencoded             |
| `Path((a,b)): Path<(A,B)>` | multiple path args | one per tuple element          |
| `Query(q): Query<T>`     | query arg      | `T` serialized to `?k=v`             |
| `Json(b): Json<T>` / `Form<T>` | body     | DTO **directly** (no `{ "b": ... }`) |
| anything else (`User`, `Session`, `State<_>`) | server extractor | stripped from TS |

Return type: `Result<T, E>` and any `*Result<T, ..>` alias (e.g.
`CollabResult<T>`) unwrap to `T`; `()`/`Result<(), _>` → `void` (no body parse).

## What this fixes vs the spike

- **Type-based classify** — no longer matches arg *names* against the route;
  reads `Path`/`Query`/`Json` from the signature → immune to rename drift.
- **`*Result` aliases** — `CollabResult<X>` now unwraps to `X` (the spike
  rendered the alias name as a bogus TS type).
- **Body un-wrapped (D2)** — request body is the DTO JSON itself.
- **Unit/204** — `Result<(), _>` → `Promise<void>`; runtime treats 204/empty
  as `undefined`.
- **Loud validation** — `Json` body on GET/DELETE, two bodies, or a
  path-placeholder/arg-count mismatch is a compile error.
- **Pure axum** — expands to inventory-registered axum routes; no
  `dioxus::fullstack`.

## Known limits

- Wrapper detection is by **last path segment ident** — import `Path`/`Query`/
  `Json` unaliased (`use axum::extract::Path as P` is not recognized).
- DTO type *files* (`../types/*`) are produced by `ts-rs` on the DTOs; this
  crate only references them by name. `ts-rs`'s serde-compat caveats
  (`flatten`, custom (de)serialize) apply to those files.
- The generated fn ↔ `runtime/client.ts` form one wire contract; the macro's
  snapshot tests pin the generated call shape — change both together.

## Test

```bash
cargo test -p ts-server-fn        # classify + TS render (incl. validation errors)
cargo test -p ts-server-fn-axum   # end-to-end: macro → axum routes compile + register
```
