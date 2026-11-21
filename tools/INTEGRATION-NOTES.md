# Integration notes

Known items to resolve when the backend and frontend land. Kept here so nothing
is lost between agents.

## 1. Tauri argument binding for the two request-body commands

`src/ipc.ts` currently spreads the request into the invoke arguments:

```ts
invoke(name, { ...request })
```

Tauri v2 binds invoke arguments to command parameters **by name**. So:

| Command | Rust signature | Invoke args that bind |
| --- | --- | --- |
| `add_download` | `fn add_download(req: AddDownloadRequest)` | `{ req: { url, ... } }` |
| `control_download` | `fn control_download(req: ControlRequest)` | `{ req: { task_id, action, ... } }` |
| `list_tasks` | `fn list_tasks(limit: i64)` | `{ limit }` |
| `get_task` | `fn get_task(task_id: String)` | `{ task_id }` |
| `delete_schedule` | `fn delete_schedule(id: String)` | `{ id }` |

Everything else in `ipc.ts` already matches. Fix: wrap only `add_download` and
`control_download` in `{ req: request }`. Do not change the Rust signatures —
they are the cleaner side of the contract.

## 2. Model routing actually used

The project file asked for three specific routes. Reality on this machine:

| Slot | Requested | Used | Why |
| --- | --- | --- | --- |
| Main | tokensbd / deepseek-v4.1-flash-1017 | nous / longcat-2.5:free (unchanged, per the file's "do not change my main agent's model") | — |
| Subagent 1 | toptoolsai / Top-Tools-Ai-Max, fallback hcnsec / step-5-preview | **nous / longcat-2.5:free** | `Top-Tools-Ai-Max`: "billing, credits, or account entitlement is exhausted". `step-preview` on hcnsec fails on the *second* turn of an agent session with `HTTP 400: duplicate field reasoning_content` — the provider returns a reasoning field Hermes's OpenAI client cannot re-serialize into the next request. `--reasoning none` works for a single generation but not for a multi-turn agent loop. `tokens.bd` works but took 870 s for one 600-word turn. The file's own instruction applies: "If separate routing is unavailable, explain the exact limitation and use available safe alternatives rather than pretending the routing works." |
| Subagent 2 | agentrouter / gpt-6-astra | agentrouter / gpt-6-astra | as requested |

## 3. Verification status

- Fixture server self-test: 46/46 pass.
- `cargo check`, `cargo test`, `npm run build`, `npm run tauri build`: pending.
