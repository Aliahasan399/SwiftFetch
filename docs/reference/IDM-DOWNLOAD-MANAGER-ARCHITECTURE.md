# Download Manager Enterprise Architecture Specification

**Product name:** Working title: `SwiftFetch`  
**Product category:** IDM-style desktop download manager  
**Primary platform:** Windows 10/11 x64; later portability is possible  
**Audience:** Executive leadership, product owner, security reviewer, and downstream development agents  
**Document status:** Architecture baseline for a new product  
**Prepared:** 2026-11-09  

> This document is for a download manager inspired by the category of Internet Download Manager. It is not a copy of IDM, does not use IDM branding, and does not bypass DRM, paywalls, authentication controls, or website restrictions. The product must use its own name, code, user interface, and implementation.

## 1. Executive decision

Build a **Windows-first, local desktop download manager** as a modular monolith with a native download engine, a desktop presentation layer, browser extensions, and encrypted local state. The product should make legitimate downloads faster and more reliable through segmented HTTP range requests, pause/resume, queueing, scheduling, retry recovery, and browser integration.

The first release should not become a distributed cloud platform. It does not need Kubernetes, Kafka, GraphQL federation, multi-region replication, CRDT collaboration, a hosted vector database, or a mandatory online account. Those systems solve multi-user service problems, while the core product is a local utility whose quality is measured by download correctness, throughput, stability, privacy, and ease of use.

### 1.1 Product promise

> Add a permitted download once, and the application manages the transfer safely, efficiently, resumably, and transparently until the file is complete.

### 1.2 Recommended product shape

```text
Browser extensions / clipboard / drag-and-drop / file associations
                              |
                              v
+-------------------------------------------------------------+
| Desktop application                                         |
|                                                             |
|  React/TypeScript UI in Tauri                              |
|       | typed IPC                                          |
|  Rust application core                                     |
|       |                                                     |
|  Download scheduler -> HTTP engine -> segment workers       |
|       |                    |                                |
|  SQLite state           disk writer / assembler              |
|       |                    |                                |
|  Windows OS adapters     .part and final files                |
+-------------------------------------------------------------+
                              |
                 Optional local antivirus integration
```

### 1.3 Critical truth about speed

The application can improve throughput when the server supports range requests and has unused capacity, but it cannot exceed the user's internet connection, the server's available bandwidth, the storage device, or the site's policies. Multi-connection downloading is an optimization, not a guarantee. The product must report actual measured throughput and never promise a universal “500% faster” result.

## 2. Scope, assumptions, and legal boundary

### 2.1 In scope for the first production release

- HTTP and HTTPS downloads.
- Optional FTP support only after a separate security and maintenance review.
- Segmented downloads when the server supports `Range` requests.
- Single-stream fallback when segmentation is unavailable.
- Pause, resume, cancel, retry, and recover after application restart.
- Download queue, priorities, categories, and per-task/global speed limits.
- Scheduled start and stop, including optional shutdown after completion.
- Browser extension handoff for permitted downloadable resources.
- Clipboard and drag-and-drop URL capture.
- Automatic file naming and category-based destination folders.
- Checksum verification when a trusted checksum is supplied.
- Optional antivirus scan through the operating system or user-configured scanner.
- Dark mode, keyboard operation, localization-ready UI, and accessible status messages.
- Signed Windows installer, versioned updates, rollback, and local diagnostics.

### 2.2 Deferred or optional

- Torrent/P2P support.
- Site-wide crawling or mirroring.
- Video extraction from adaptive streaming manifests.
- Mobile companion or remote control.
- Cloud synchronization of queues.
- Team administration and centralized policy.
- macOS/Linux builds.

These features are not automatically permitted by the architecture. Each adds legal, security, protocol, or support obligations.

### 2.3 Explicitly prohibited

- DRM circumvention.
- Paywall or subscription bypass.
- Password or authentication bypass.
- Evading a website's robots, rate, access, or anti-abuse controls.
- Credential scraping without explicit user consent.
- Hidden background downloads.
- Bundling unrelated software or changing browser settings without consent.
- Executing downloaded files automatically.
- Sending download URLs, cookies, filenames, or file contents to a vendor server by default.

## 3. The 12-layer decision matrix

The original master prompt says “12 layers” but lists only 11 numbered layers. This specification adds Layer 12 for governance, operations, continuity, and cost.

| Layer | Decision | Priority | Current product treatment |
| --- | --- | --- | --- |
| 1. Presentation/frontend | Adopt selectively | Core | One desktop UI module system; no micro-frontends or SDUI |
| 2. Backend/business logic | Adopt selectively | Core | Rust hexagonal core, scheduler, transfer engine, and state machine |
| 3. Data/storage | Adopt selectively | Core | Local SQLite with WAL; no vector or time-series database |
| 4. Integration ecosystem | Adapt | Core | Browser extensions, OS shell, antivirus, and optional checksum sources |
| 5. Security/governance/compliance | Adopt | Core | URL/input safety, secret handling, signed updates, privacy, legal boundaries |
| 6. Infrastructure/DevOps | Adapt | Core for delivery | Desktop CI, protocol tests, fault injection, signing; no Kubernetes |
| 7. Local data management | Adopt | Core | Durable queue, segment metadata, crash recovery, atomic finalization |
| 8. Hardware/OS abstraction | Adopt | Core | Windows filesystem, process, power, network, and notification adapters |
| 9. Networking/sync | Adopt selectively | Core | HTTP/HTTPS range transfers; no multi-device sync initially |
| 10. Licensing/runtime protection | Adapt | Release/business | Signed licensing optional; do not use invasive anti-debugging |
| 11. Update/telemetry/packaging | Adopt selectively | Release-critical | Signed installer, rollback, privacy-preserving local diagnostics |
| 12. Governance/operations/cost | Adopt | Executive control | Ownership, support, release policy, legal review, cost model |

## 4. Executive requirements

### 4.1 Business outcomes

| Outcome | Measure |
| --- | --- |
| Faster permitted downloads | Compare effective throughput against a single-stream baseline on the same URL and network |
| Fewer failed downloads | Completion rate across interrupted, resumed, and server-error fixtures |
| Better user control | Queue, priority, schedule, speed limit, pause, and retry actions are discoverable and reliable |
| Privacy | No default upload of URLs, cookies, filenames, or file contents |
| Trust | Signed installer, transparent permissions, safe updates, and clear legal boundary |
| Low operating cost | Core download behavior works without a hosted service or per-download cloud fee |

### 4.2 Non-functional targets

These are targets for measurement, not promises to users until acceptance evidence exists.

| Area | Initial target |
| --- | --- |
| UI startup | Shell interactive within 1 second on the baseline Windows device; model is not applicable |
| Add-download response | URL accepted or rejected within 200 ms for normal local validation |
| Scheduler decision | Queue state recalculated within 100 ms after a task event |
| Disk behavior | Stream to disk; no requirement to buffer the complete file in memory |
| Segmentation | Default 4–8 connections per eligible task; configurable safety ceiling |
| Global concurrency | Bounded by user setting and resource policy; never unlimited |
| Resume | Preserve verified completed segments and restart only invalid portions |
| Crash recovery | Reopen queue and recover durable tasks after application restart |
| Integrity | Optional cryptographic hash verification before completed status |
| Offline usability | Existing tasks, history, settings, and completed files remain usable offline |
| Accessibility | WCAG 2.1 AA baseline for the desktop UI; AAA where practical and evidenced |
| Privacy | No telemetry or cloud account required for core operation |

## 5. System context and trust boundaries

### 5.1 Context diagram

```text
 +-------------------+       explicit handoff       +----------------------+
 | Chrome / Edge /   | ---------------------------> | Browser bridge       |
 | Firefox extension |                             | native messaging or  |
 +-------------------+                             | authenticated local  |
                                                     | channel              |
 +-------------------+       URL / file             +----------+-----------+
 | User              | --------------------------->            |
 | clipboard, files, |                                      v
 | drag-and-drop     |                             +----------------------+
 +-------------------+                             | Download manager     |
                                                     |                      |
                                                     | UI + Rust core       |
                                                     | Scheduler            |
                                                     | HTTP transfer engine |
                                                     | Local state store    |
                                                     +----+------------+----+
                                                          |            |
                                                   network|            |disk
                                                          v            v
                                                +---------+--+   +-----+------+
                                                | HTTP/HTTPS |   | User files |
                                                | servers    |   | .part/final|
                                                +------------+   +------------+
```

### 5.2 Trust-boundary table

| Boundary | Untrusted input | Required control |
| --- | --- | --- |
| Browser → app | URL, title, suggested filename, cookies | User consent, scheme validation, bounded fields, no command execution |
| UI → core | IDs, paths, limits, actions | Typed IPC, authorization by task ownership, bounds, canonical paths |
| Server → HTTP engine | Headers, redirects, content length, bytes | Header parser limits, redirect policy, content validation, timeout, hash check |
| Disk → app | Existing `.part` files and metadata | Validate task identity, size, offsets, hash, and path before reuse |
| Downloaded file → OS | Potentially unsafe content | Never auto-execute; optional antivirus scan; user confirmation |
| Update source → installer | Package, manifest, signature | Verify product identity, signature, hash, version, and rollback path |

## 6. Layer 1 — Complex presentation and frontend

### 6.1 Architecture decision

Use a **single desktop frontend with feature modules** in Tauri. The UI can use React and TypeScript, but the download engine and security policies must remain in Rust or another native core. Do not use Module Federation or server-driven UI in the first release.

Micro-frontends are unnecessary because the product is installed as one desktop application, has one release train, and has no independent business teams delivering remote modules. Server-driven UI would undermine offline startup and introduce a remote UI supply-chain boundary.

### 6.2 Frontend module map

```text
ui/
  shell/              window, navigation, tray, global notifications
  dashboard/          active downloads and aggregate throughput
  queue/              ordering, priority, pause/resume, bulk actions
  add-download/       URL, filename, destination, authentication consent
  task-details/       segments, headers summary, retry history, hash status
  scheduler/           time windows, queue rules, shutdown actions
  categories/          extension/site rules and destination folders
  settings/            connection, proxy, privacy, browser integration
  history/             completed/failed tasks and search
  diagnostics/         redacted local support bundle
  shared/              typed contracts, design system, accessibility helpers
```

### 6.3 State management

Use a reactive store for projections of core state. The UI is not the source of truth; it subscribes to events and requests commands.

```ts
type DownloadStatus =
  | "queued"
  | "starting"
  | "downloading"
  | "paused"
  | "retry_wait"
  | "verifying"
  | "scanning"
  | "completed"
  | "failed"
  | "cancelled";

type DownloadProjection = {
  id: string;
  displayName: string;
  status: DownloadStatus;
  bytesReceived: number;
  totalBytes: number | null;
  bytesPerSecond: number;
  etaSeconds: number | null;
  queueId: string;
  priority: number;
  canResume: boolean;
  errorCode?: string;
};
```

Optimistic state is acceptable for a button press while awaiting a core acknowledgement, but the UI must reconcile to the event from Rust. Never show a file as completed before verification and atomic finalization succeed.

CRDTs, collaborative editing, and operational transformation are not needed. There is no shared mutable document or team workspace in the current product.

### 6.4 Accessibility and internationalization

- Every download action must be keyboard reachable.
- Progress must have accessible text, not only a moving bar.
- Status changes should be announced through a throttled live region.
- Color must not be the only indicator of failed, paused, or completed state.
- Long filenames must have accessible full-name tooltips and a copy action.
- Dialogs for credentials, overwrite, and dangerous file actions require focus trapping and explicit cancel paths.
- Use locale-aware dates, times, decimal separators, byte units, and pluralization.
- Keep layout direction abstract to support RTL later.
- Persist locale keys, not translated strings.

### 6.5 Performance and long-session safety

- Virtualize large queue and history lists.
- Throttle high-frequency progress events before they reach React rendering.
- Store byte counters as integers and format only at the view boundary.
- Unsubscribe from task streams when a view unmounts.
- Cap history queries and paginate older entries.
- Do not place response bodies, cookies, authorization headers, or full local paths in global UI state unless the user is actively viewing them.
- Test a 24-hour session with thousands of completed tasks for event-listener leaks and stale timers.

### 6.6 Frontend failure behavior

| Situation | Required UI behavior |
| --- | --- |
| Network offline | Show paused/retry status; preserve queue and allow local management |
| Disk full | Show actionable error with path and free-space estimate; do not delete data automatically |
| Server refuses ranges | Explain that single-stream mode is being used |
| Resume metadata invalid | Ask before restarting from zero when data may be overwritten |
| Antivirus scan fails | Mark as scan-incomplete; never silently mark trusted |
| App restarts during transfer | Restore task state and show recovery result |
| Browser bridge unavailable | Preserve the URL and show manual-add instructions |

## 7. Layer 2 — Complex backend and business logic

### 7.1 Architecture decision

Use a **hexagonal modular core** with explicit domain services. A single process is sufficient for the first release, but the transfer engine should be separated by interfaces so it can be tested without the UI.

```text
Tauri commands / browser bridge / OS events
                      |
                      v
Application services
  AddDownload, ControlDownload, QueueScheduler,
  ImportSettings, VerifyFile, ManageBrowserIntegration
                      |
                      v
Domain policies and state machines
  DownloadTask, Segment, Queue, Category, Schedule, RetryPolicy
                      |
                      v
Ports
  HttpTransport, DiskStore, MetadataStore, Clock,
  ProcessRunner, AntivirusScanner, NotificationSink
                      |
                      v
Adapters
  HTTP client, SQLite, Windows filesystem, Windows APIs,
  browser native messaging, optional scanner integration
```

### 7.2 Bounded contexts

| Context | Owns |
| --- | --- |
| Download catalog | URL identity, filename, destination, task status |
| Transfer engine | HTTP requests, ranges, segment workers, throughput |
| Queue scheduler | concurrency, priorities, schedules, global limits |
| File finalization | temporary files, assembly, overwrite policy, atomic rename |
| Browser integration | extension handoff, consent, browser registration |
| Settings | proxy, limits, categories, privacy, feature flags |
| Verification | checksums, content-length reconciliation, optional scan |
| Licensing | local license state if the commercial model requires it |

### 7.3 Download state machine

```text
NEW
 | add/validate
 v
QUEUED --start--> STARTING --eligible--> DOWNLOADING
  ^                  |  |                  |
  |                  |  +--no range------->| single stream
  |                  |                     |
  |                  +--failure--> RETRY_WAIT --timer--> STARTING
  |                                        |
  |                                        +--permanent--> FAILED
  |
  +--pause/cancel <------------------------+

DOWNLOADING --pause--> PAUSED --resume--> STARTING
DOWNLOADING --bytes complete--> VERIFYING --valid--> SCANNING/FINALIZING
VERIFYING --invalid--> FAILED or restart-required
FINALIZING --atomic rename--> COMPLETED
```

### 7.4 Transfer algorithm

1. Validate URL scheme and normalized URL.
2. Apply redirect policy, maximum redirect count, and allowed scheme rules.
3. Perform a bounded metadata request or initial GET.
4. Record response validators: `Content-Length`, `Accept-Ranges`, `ETag`, `Last-Modified`, `Content-Type`, and final URL.
5. If the server supports byte ranges and the size is known, divide the file into bounded ranges.
6. Create or reuse a `.part` file and durable segment metadata.
7. Run a bounded number of segment workers. Each worker requests one range and writes only to its assigned offset or segment file.
8. On response mismatch, retry with backoff or fall back to a safe single-stream strategy.
9. Periodically persist progress without flushing on every packet.
10. When all segments are complete, verify size and optional checksum.
11. Run the optional antivirus scanner.
12. Atomically rename the verified temporary file to the final destination according to the overwrite policy.

### 7.5 Segmentation rules

- Do not open multiple connections when the server rejects or ignores range requests.
- Do not treat a `200 OK` response to a ranged request as a valid range response.
- Validate `Content-Range` against the requested offset and length.
- Bound the number of segments per task and globally.
- Adapt down on `429`, repeated `5xx`, connection resets, or rising error rate.
- Reuse an existing connection only when the HTTP client can prove the response belongs to the intended range.
- Do not bypass server-imposed rate limits.
- A task should have a user-visible connection count and an emergency “single connection” mode.

### 7.6 Retry and backoff

```text
Retryable: timeout, connection reset, DNS transient failure, 408, 425, 429, 500, 502, 503, 504
Not automatically retryable: invalid URL, unsupported scheme, 401/403 without user action,
404, 410, checksum mismatch after bounded attempts, disk permission failure
```

Use exponential backoff with jitter and a ceiling. Honor `Retry-After` when valid and bounded. Never retry an operation that may duplicate a completed file write without checking the segment checkpoint.

### 7.7 Local Saga for a download

The product does not need distributed transactions. A local download workflow is still a compensating sequence:

```text
Create task record
  -> create private .part path
  -> persist segment plan
  -> start workers
  -> persist checkpoints
  -> verify bytes/hash
  -> scan if enabled
  -> atomic rename
  -> mark completed

Failure/cancel:
  -> stop workers
  -> persist safe checkpoint
  -> retain or remove .part according to user policy
  -> mark paused/failed/cancelled
  -> release scheduler slot
```

## 8. Layer 3 — Data and storage

### 8.1 Architecture decision

Use **SQLite with WAL** for local metadata and queue durability. Store large file content on the user's chosen filesystem, not inside the database. Do not use a vector database or time-series database for the core download manager.

### 8.2 Logical schema

The following schema is a design contract. Migration names and implementation details must be reviewed before coding.

```sql
CREATE TABLE queues (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  max_active_tasks INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE categories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  destination_template TEXT NOT NULL,
  extension_rules_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE downloads (
  id TEXT PRIMARY KEY,
  queue_id TEXT NOT NULL REFERENCES queues(id),
  category_id TEXT REFERENCES categories(id),
  original_url TEXT NOT NULL,
  final_url TEXT,
  display_name TEXT NOT NULL,
  destination_path TEXT NOT NULL,
  temporary_path TEXT NOT NULL,
  status TEXT NOT NULL,
  total_bytes INTEGER,
  received_bytes INTEGER NOT NULL DEFAULT 0,
  etag TEXT,
  last_modified TEXT,
  accept_ranges INTEGER,
  content_type TEXT,
  checksum_algorithm TEXT,
  checksum_expected TEXT,
  checksum_actual TEXT,
  retry_count INTEGER NOT NULL DEFAULT 0,
  last_error_code TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE download_segments (
  id TEXT PRIMARY KEY,
  download_id TEXT NOT NULL REFERENCES downloads(id),
  segment_index INTEGER NOT NULL,
  start_offset INTEGER NOT NULL,
  end_offset INTEGER NOT NULL,
  received_bytes INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  UNIQUE(download_id, segment_index)
);

CREATE TABLE schedules (
  id TEXT PRIMARY KEY,
  queue_id TEXT NOT NULL REFERENCES queues(id),
  days_mask INTEGER NOT NULL,
  start_local_time TEXT NOT NULL,
  end_local_time TEXT NOT NULL,
  action_after TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE event_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  download_id TEXT,
  event_type TEXT NOT NULL,
  safe_payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

### 8.3 File layout

```text
User-selected destination/
  file.zip                         completed file
  file.zip.part                    temporary transfer file, if configured

App data/
  state.sqlite                     queue and settings metadata
  state.sqlite-wal                 SQLite write-ahead log
  state.sqlite-shm                 SQLite shared-memory file
  downloads/                       optional private working files
  diagnostics/                     bounded local redacted reports
```

The application must never infer that a file is complete merely because a `.part` file exists. Completion requires durable metadata, expected size reconciliation, and optional verification.

### 8.4 Data lifecycle

| Data | Default retention | User control |
| --- | --- | --- |
| Active task metadata | Until completion or user deletion | Pause, retry, delete task, retain file |
| Completed history | Configurable; default 30–90 days | Clear history without deleting files |
| URLs and headers | Local only; retained with task as needed | Clear history/settings |
| Cookies/credentials | Memory-only where possible; encrypted local store only if explicitly enabled | Revoke/clear integration data |
| Diagnostic events | Bounded local ring buffer | Inspect and delete/export |
| Partial files | Until completed, cancelled, or user chooses retention | Keep/resume/delete |

### 8.5 Migrations and backup

- Use monotonic, idempotent schema migrations.
- Back up the local metadata database before a destructive migration.
- Never delete user files during a schema migration.
- Restore into a new database first, run integrity checks, then activate.
- Keep queue state separate from installation files so update/uninstall does not erase it.
- Provide an export of queue metadata without exporting cookies or secrets by default.

### 8.6 Disaster recovery

There is no multi-region database in the desktop product. The practical targets are:

| Metric | Initial target |
| --- | --- |
| RPO | Last successful local metadata backup; default 24 hours if scheduled backup is enabled |
| RTO | Restore queue/settings metadata within 30 minutes on a compatible Windows machine |
| File recovery | Existing completed files remain user-owned and are never overwritten by restore without confirmation |
| Partial recovery | Reconcile `.part` files against segment metadata and server validators |

## 9. Layer 4 — Integration and third-party ecosystem

### 9.1 Architecture decision

Use narrow, explicit integrations rather than an enterprise service bus:

```text
Browser extension
  -> validated download-intent message
  -> native messaging host or authenticated loopback bridge
  -> Rust core

OS shell/clipboard/drag-drop
  -> Tauri/Windows adapter
  -> Rust core

Optional antivirus/checksum provider
  -> reviewed adapter interface
  -> verification service
```

Kafka, RabbitMQ, ESB, GraphQL federation, and gRPC streaming are unnecessary for the local app.

### 9.2 Browser extension contract

The extension must capture only an explicit user action or a user-enabled site rule. It must not silently inspect or transmit all browsing traffic.

```json
{
  "protocol": 1,
  "type": "download_intent",
  "request_id": "uuid",
  "url": "https://example.com/file.zip",
  "suggested_name": "file.zip",
  "page_origin": "https://example.com",
  "content_type": "application/zip",
  "user_initiated": true,
  "cookies": [],
  "expires_at": "2026-11-09T12:00:00Z"
}
```

Rules:

- Accept only `http` and `https` unless a separate protocol is explicitly enabled.
- Validate URL length and reject control characters.
- Treat the suggested filename as untrusted text.
- Never log cookies or authorization headers.
- Require user confirmation before sending sensitive credentials or private downloads.
- Reject expired or replayed intents.

### 9.3 External provider resilience

External services are optional. If checksum metadata or an antivirus adapter is unavailable, the download can remain usable but must display that verification was not performed. No provider failure should delete the downloaded file.

## 10. Layer 5 — Security, governance, and compliance

### 10.1 Security model

Use a local zero-trust approach:

- The UI is not trusted with unrestricted filesystem access.
- Browser input is not trusted with command execution.
- HTTP response headers are not trusted with unbounded resource values.
- Downloaded files are not trusted executable content.
- Update packages are not trusted until signature and product identity verification succeeds.
- Cookies and credentials are secrets and must not enter logs, URLs, crash dumps, or analytics.

OAuth/OIDC, RBAC/ABAC, MFA, HSMs, and enterprise IAM are not required for the single-user offline release. They become relevant only if a hosted account or centralized administration is added.

### 10.2 URL and redirect policy

```text
Allowed by default: http://, https://
Rejected by default: file://, javascript:, data:, blob:, custom command schemes

Redirect rules:
  - maximum redirect count: bounded configuration
  - validate each target scheme
  - do not silently downgrade HTTPS to HTTP
  - display final host when it differs materially from the entered URL
  - do not forward credentials across origins without explicit policy
```

### 10.3 Path and filename safety

- Resolve destination paths against the selected download directory.
- Reject path traversal, device names, reserved names, and invalid Windows path forms.
- Normalize Unicode consistently before collision checks.
- Do not use the filename as a shell command or process argument without a fixed argument boundary.
- Apply a user-visible overwrite policy: rename, replace after confirmation, skip, or resume.
- Ensure the temporary file and final file remain within the intended destination root.

### 10.4 Secret handling

Possible secrets include HTTP Basic credentials, cookies, proxy credentials, and license tokens.

- Prefer OS credential storage or DPAPI-protected storage.
- Keep secrets out of the frontend store whenever possible.
- Redact secrets from logs, error messages, diagnostic bundles, and screenshots.
- Never include credentials in a URL saved to history.
- Provide a clear “clear browser integration data” action.

### 10.5 Downloaded-file safety

- Never execute a downloaded file automatically.
- Use a safe temporary extension if a platform integration could misinterpret the file.
- Verify expected size and optional checksum.
- Allow optional antivirus scanning before the completed state.
- Show the exact final path and verification status.
- Do not claim that a checksum or antivirus scan proves a file is safe in every sense.

### 10.6 Compliance and legal position

The software can support privacy and security controls, but it cannot itself guarantee GDPR, HIPAA, SOC 2, ISO 27001, or CCPA compliance. Compliance depends on the business, data, deployment, policies, and legal basis.

| Topic | Product control | Still required from the business |
| --- | --- | --- |
| Privacy | No default cloud upload; local history controls | Privacy notice, lawful basis, support process |
| Copyright | Explicit no-bypass boundary and user confirmation | Terms, moderation/support process, legal review |
| Security | Signed binaries, safe paths, secret redaction | Vulnerability response and release governance |
| Enterprise use | Local policy hooks can be added | Procurement, admin controls, audit evidence |

### 10.7 Auditability

Keep a bounded local event record for security and troubleshooting, not a full surveillance log.

```json
{
  "event": "download_failed",
  "task_id_hash": "sha256:...",
  "code": "HTTP_403",
  "host_hash": "sha256:...",
  "bytes_received": 1048576,
  "timestamp": "2026-11-09T12:00:00Z",
  "url": "[redacted]"
}
```

## 11. Layer 6 — Infrastructure, DevOps, and delivery

### 11.1 Architecture decision

Use a desktop release pipeline, not cloud-native deployment infrastructure.

```text
Commit
  -> dependency lock and license scan
  -> Rust unit/integration tests
  -> frontend lint/typecheck/test
  -> protocol contract tests
  -> HTTP fixture-server tests
  -> fault-injection and disk tests
  -> Windows build
  -> installer build
  -> signature/hash/manifest generation
  -> clean-machine install test
  -> staged release
```

### 11.2 Required test environment

The automated test suite needs local deterministic HTTP fixtures capable of returning:

- normal single-stream content;
- valid range responses;
- ignored range headers;
- malformed `Content-Range`;
- short body and mismatched `Content-Length`;
- redirects and redirect loops;
- 401, 403, 404, 408, 429, and 5xx responses;
- `Retry-After` headers;
- connection interruption at known byte offsets;
- slow responses and delayed headers;
- checksum mismatch;
- changed ETag/Last-Modified during resume.

### 11.3 Verification gates

| Gate | Acceptance |
| --- | --- |
| Unit | URL, filename, range, retry, schedule, state-machine, and limit tests pass |
| Integration | Real local fixture transfers, pause/resume, restart recovery, and finalization pass |
| Browser contract | Extension messages reject malformed, unauthorized, expired, and replayed intents |
| Security | Path traversal, scheme validation, secret redaction, update signature, and no-auto-exec tests pass |
| Resource | Large-file streaming does not scale memory with file size |
| Performance | Throughput and UI responsiveness measured on baseline hardware |
| Clean install | Installer, first launch, upgrade, rollback, and uninstall preservation pass |
| Offline | Queue/history/settings remain usable without network; active transfers show recoverable state |

### 11.4 Observability

The production app should not require Prometheus, Grafana, Splunk, Datadog, or OpenTelemetry collectors. Use bounded local metrics and optional user-exported diagnostics.

Useful local metrics include:

- effective bytes per second;
- time to first byte;
- connection count;
- segment retry count;
- response status distribution by code, not full URL;
- disk-write latency;
- queue wait time;
- resume success rate;
- checksum and scan outcomes.

Default diagnostics must exclude URLs, filenames, cookies, authorization headers, and file contents.

## 12. Layer 7 — Complex local data management and storage

### 12.1 Durable queue and write-ahead behavior

The queue must survive process termination and power loss. SQLite WAL protects metadata transactions; the file transfer engine protects file content through segment checkpoints and atomic finalization.

```text
Network bytes
  -> bounded buffer
  -> segment writer
  -> flush policy
  -> segment checkpoint transaction
  -> scheduler projection update
```

Do not flush metadata on every packet. Do not acknowledge durable progress before the corresponding file bytes are safely written according to the chosen durability policy.

### 12.2 Segment checkpoint contract

```json
{
  "task_id": "uuid",
  "segment_id": "uuid",
  "requested_start": 0,
  "requested_end": 1048575,
  "written_until": 524287,
  "etag": "opaque-validator",
  "last_modified": "opaque-validator",
  "file_identity": "sha256-of-path-and-size-policy",
  "updated_at": "2026-11-09T12:00:00Z"
}
```

On resume, the engine must check that the server representation is compatible. If validators changed, the safe default is to restart affected data or ask the user rather than silently combine different versions.

### 12.3 Cache and temporary data

- Keep HTTP response buffers bounded.
- Do not cache complete bodies in memory.
- Keep temporary files beside or under the destination according to the user's atomicity/space preference.
- Use unique task-scoped temporary names.
- Remove abandoned temporary files only after showing or recording what will be removed; never delete unrelated files by pattern alone.
- Keep recent error metadata, not entire response bodies.

### 12.4 Disk-full and interruption behavior

If disk space becomes insufficient:

1. stop writing the affected segment;
2. cancel or pause peer segments for the same task;
3. persist the last safe checkpoint;
4. preserve the partial file if it is resumable;
5. release the scheduler slot;
6. explain the required free space and target path.

## 13. Layer 8 — Hardware and OS abstraction

### 13.1 Architecture decision

Use a native adapter boundary for Windows features:

```text
Core policies
  -> OS ports
     -> Windows implementation
        - atomic rename and file locks
        - free-space and volume information
        - system tray and notifications
        - shutdown/sleep request
        - network availability
        - proxy and credential store
        - process/job lifetime if helper processes exist
```

### 13.2 CPU, memory, disk, and network policy

- The engine is I/O-bound for most downloads; optimize allocations and disk writes before adding GPU support.
- Keep per-task buffers bounded.
- Limit simultaneous tasks and segment workers based on user settings and measured resource pressure.
- Detect slow disks and reduce concurrency when write latency threatens correctness.
- Prefer a stable throughput curve over aggressive connection spikes.
- Respect system metered-network and battery signals where available.
- Pause scheduled bulk work when the device is on battery if the user enables that policy.

GPU acceleration is not needed for ordinary downloads. CUDA/Metal hooks should not be added to the first release.

### 13.3 Power and sleep behavior

The user must explicitly choose whether active downloads prevent sleep. The app should:

- show the current power policy;
- release the power request after all relevant tasks stop;
- restore the previous system state after exit;
- avoid preventing sleep for a paused or completed queue.

### 13.4 Cross-platform strategy

Keep the transfer engine and protocol tests platform-neutral. Isolate Windows UI, notification, power, installer, and credential adapters. A later macOS/Linux port is then a bounded adapter project rather than a rewrite.

## 14. Layer 9 — Networking, synchronization, and enterprise integration

### 14.1 Transport decision

Use a mature HTTP client with HTTP/1.1 and HTTP/2 support as appropriate. Do not implement custom TCP/UDP framing. QUIC/HTTP3 can be evaluated later if the selected client supports it safely, but it is not a first-release requirement.

| Transport | Use |
| --- | --- |
| HTTP/1.1 | Required baseline for range and single-stream transfers |
| HTTP/2 | Optional acceleration where server/client behavior is stable |
| HTTP/3/QUIC | Deferred; measure before adopting |
| FTP | Optional later adapter, separate security review |
| WebSocket | Not needed for file transfer |
| gRPC-Web | Not needed for local desktop transfer |
| Custom UDP | Prohibited for the first product |

### 14.2 Connection management

- Use per-host connection budgets.
- Honor `Retry-After` and server errors.
- Do not send unlimited parallel requests to one host.
- Use timeouts for DNS, connect, TLS handshake, headers, idle read, and total operation.
- Support proxy settings explicitly; do not silently exfiltrate through an unexpected proxy.
- Allow user cancellation to interrupt DNS, connect, read, and write paths.

### 14.3 Sync decision

No multi-device queue synchronization is required for the initial product. CRDTs, vector clocks, LWW, and operational transformation are therefore out of scope.

If remote synchronization is later requested, create a new architecture for:

- device enrollment and revocation;
- encryption and key ownership;
- conflict handling for task state;
- duplicate file identity;
- secret and cookie exclusion;
- remote deletion semantics;
- offline edits and retries;
- data residency and support access.

Do not synchronize credentials or cookies by default.

### 14.4 Adaptive bandwidth

The engine may adapt connection count and speed based on:

- user global/task limits;
- observed server throughput;
- error rate;
- metered-network state;
- other active application traffic only when the OS provides reliable signals.

It must not claim to optimize every other application’s traffic or use invasive packet inspection.

## 15. Layer 10 — Licensing, runtime protection, and compliance

### 15.1 Licensing options

| Model | Suitable use | Main concern |
| --- | --- | --- |
| Free | Prototype and adoption | Support and funding |
| Perpetual license | Offline utility | License transfer/recovery support |
| Subscription | Continuous premium features | Conflicts with offline expectations and adds account service |
| Enterprise deployment | Managed organizations | Admin policy, procurement, support, audit evidence |

For an offline-first first release, use a signed local license token or a perpetual key if commercial licensing is required. Do not make basic file recovery dependent on a live license server.

### 15.2 License token blueprint

```json
{
  "product": "swiftfetch",
  "edition": "professional",
  "license_id": "public-id",
  "issued_at": "2026-11-09T00:00:00Z",
  "expires_at": null,
  "features": ["scheduler", "browser_integration"],
  "key_id": "signing-key-id",
  "signature": "base64url-signature"
}
```

Use a reviewed signature library and rotate signing keys through a documented release process. Do not invent cryptography or use an obfuscated secret embedded in the binary.

### 15.3 Runtime protection

- Code-sign binaries and installers.
- Verify update signatures and artifact hashes.
- Protect local secrets with OS facilities.
- Treat obfuscation as a deterrent, not a security boundary.
- Avoid invasive anti-debugging that breaks support tools or accessibility.
- Do not use hardware fingerprinting as the only license or security control.
- Do not auto-start hidden processes or install services without explicit disclosure and consent.

## 16. Layer 11 — Update, telemetry, and packaging

### 16.1 Packaging

Recommended Windows artifacts:

```text
release/
  SwiftFetch-<version>-x64-setup.exe
  release-manifest.json
  SHA256SUMS
  sbom.spdx.json
  signature metadata
  release-notes.md
  browser-extension-<browser>-<version>.zip
```

The installer must state whether it is per-user or machine-wide, what permissions it requests, where data is stored, and how to uninstall without deleting downloads or queue state.

### 16.2 Safe update sequence

```text
Optional update check
  -> download package to staging
  -> verify product identity and signature
  -> verify hash and version policy
  -> stop or quiesce transfer workers safely
  -> update application files only
  -> launch health check
  -> retain rollback package until success
  -> resume queue after user-visible confirmation/policy
```

Never overwrite a user file or delete queue state as part of an application update.

### 16.3 Telemetry policy

No telemetry is required for the core product. If opt-in diagnostics are later added:

- opt-in must be explicit and reversible;
- URLs, filenames, cookies, headers, local paths, and file contents must be excluded or redacted;
- event schemas must be documented;
- the user can inspect, export, and delete queued reports;
- the app remains fully functional offline;
- telemetry failure cannot block downloads.

### 16.4 Crash handling

- Default to local, bounded crash metadata.
- Avoid full memory dumps because they may contain URLs, cookies, and file data.
- Redact command lines and environment variables.
- Provide an export preview before a diagnostic bundle leaves the machine.

## 17. Layer 12 — Governance, operations, continuity, and cost

### 17.1 Ownership model

| Responsibility | Owner | Evidence |
| --- | --- | --- |
| Download correctness | Transfer-engine maintainer | Protocol fixture suite and benchmark report |
| Browser integration | Extension maintainer | Browser contract and permission review |
| Security | Security/technical lead | Threat model and release security checklist |
| Installer and signing | Release owner | Signature verification record |
| Legal boundaries | Product/legal owner | Terms, privacy notice, permitted-use policy |
| Support | Product owner | Diagnostic bundle and recovery runbook |
| Accessibility | Frontend owner | Automated and manual accessibility review |

### 17.2 Approval matrix

| Change | Agent may prepare | Human approval required |
| --- | --- | --- |
| UI copy and documentation | Yes | Product review |
| URL/redirect policy | Prepare only | Security review |
| Cookie or credential handling | Prepare tests/design | Security and privacy approval |
| File deletion behavior | Prepare proposal | Explicit product approval |
| Database migration | Prepare migration and rollback test | Storage owner approval |
| New browser permission | Prepare manifest and rationale | Security/product approval |
| New external service | Prepare cost/privacy analysis | Business/privacy approval |
| Installer signing | No secret access | Release owner |
| License enforcement | Prepare design | Product/legal/security approval |

### 17.3 Cost model

The local-first design avoids per-download cloud compute and storage costs. Expected costs are:

| Cost | Notes |
| --- | --- |
| Engineering | Largest cost; transfer correctness and browser compatibility require ongoing testing |
| Code signing | Annual certificate and release process |
| Build/CI | Windows build runners, artifact retention, clean-machine testing |
| Website/update hosting | Optional; required only for distribution/update channel |
| Browser stores | May involve registration or review requirements |
| Support | Recovery, site compatibility, and antivirus false-positive handling |
| Compliance/legal | Terms, privacy, copyright, and enterprise procurement review |

### 17.4 Incident response

Required runbook cases:

1. Malicious or compromised release artifact.
2. Signing-key compromise.
3. Credential/cookie exposure.
4. Unexpected deletion or overwrite of a user file.
5. Browser-extension permission or supply-chain issue.
6. Downloaded content falsely treated as verified.
7. Website compatibility regression causing abusive request volume.

Each incident requires containment, affected-version identification, a user-safe recovery path, and a documented release decision.

## 18. API and protocol contracts

These contracts are implementation targets for a new application. Downstream agents must keep them versioned and must not silently rename fields between UI, browser, and core.

### 18.1 Add-download command

```json
{
  "protocol": 1,
  "request_id": "uuid",
  "url": "https://example.com/archive.zip",
  "suggested_name": "archive.zip",
  "destination_directory": "C:/Users/Ali/Downloads",
  "queue_id": "default",
  "priority": 50,
  "start_policy": "immediate",
  "headers": [],
  "credentials_reference": null,
  "checksum": {
    "algorithm": "sha256",
    "value": null
  }
}
```

Validation:

- URL scheme is allowed.
- URL and name length are bounded.
- destination is canonicalized and permitted.
- headers are an allowlisted structure and cannot inject commands.
- credentials require explicit user consent.
- priority, queue, and start policy are valid values.

### 18.2 Download event

```json
{
  "protocol": 1,
  "event": "download_progress",
  "task_id": "uuid",
  "status": "downloading",
  "received_bytes": 5242880,
  "total_bytes": 52428800,
  "bytes_per_second": 1048576,
  "active_connections": 4,
  "eta_seconds": 45,
  "timestamp": "2026-11-09T12:00:00Z"
}
```

Progress events are advisory projections. Terminal state is accepted only from the core state machine.

### 18.3 Control command

```json
{
  "protocol": 1,
  "request_id": "uuid",
  "task_id": "uuid",
  "action": "pause"
}
```

Allowed actions: `start`, `pause`, `resume`, `cancel`, `retry`, `verify`, `open_file`, `open_folder`, and `remove_history`. `delete_file` must be a separate explicit command with confirmation and a clear target path.

### 18.4 Error envelope

```json
{
  "ok": false,
  "request_id": "uuid",
  "error": {
    "code": "SERVER_DOES_NOT_SUPPORT_RANGES",
    "message": "The server does not support segmented downloading.",
    "retryable": false,
    "user_action": "CONTINUE_SINGLE_STREAM"
  }
}
```

Error messages must not include cookies, authorization headers, full sensitive URLs, or local secrets.

### 18.5 HTTP fixture contract for tests

The test fixture server should expose deterministic routes such as:

```text
/bytes/<size>                  normal body
/range/<size>                  correct ranges
/ignore-range/<size>           always 200 full body
/bad-content-range             malformed range response
/disconnect/<offset>           closes connection at offset
/status/<code>                 controlled HTTP status
/redirect/<count>              redirect chain
/etag/<version>                validator changes between runs
/slow/<delay-ms>               delayed headers/body
/checksum/<expected>           known good or bad body
```

The application must use these only in tests and must never ship test endpoints.

## 19. Failure-mode and recovery matrix

| Failure | Detection | Safe response | User-visible result |
| --- | --- | --- | --- |
| Server ignores ranges | Status/body mismatch | Switch to single stream | “Segmented mode unavailable” |
| Connection reset | Read failure | Persist checkpoint, retry | Retry countdown |
| 429 | Status and optional `Retry-After` | Reduce concurrency, back off | Rate-limited status |
| URL expired | 401/403 or signed URL error | Ask for refreshed link | Authentication/expiry action |
| Content changed | ETag/Last-Modified mismatch | Restart affected data or ask | Resume compatibility warning |
| Disk full | Write failure/free-space check | Pause and retain partial | Free-space instruction |
| Permission denied | File open/rename failure | Stop without deletion | Choose another folder |
| Hash mismatch | Verification result | Mark failed; do not complete | Integrity failure |
| App crash | Startup reconciliation | Recover durable segments | Recovery summary |
| Antivirus unavailable | Scanner adapter error | Mark scan incomplete | User chooses continue/stop |
| Browser bridge replay | Expired/request ID duplicate | Reject | Retry from browser action |
| Update signature failure | Signature/hash check | Do not install | Release rejected |

## 20. What is important and what is not needed

### 20.1 Must-have architecture

- Reliable HTTP/HTTPS engine.
- Correct range segmentation and single-stream fallback.
- Durable segment checkpoints.
- Queue and scheduler.
- Pause/resume/retry/cancel.
- Safe path and filename handling.
- Browser integration with explicit permissions.
- Speed limits and concurrency limits.
- Checksum and optional antivirus integration.
- Signed installer and safe updates.
- Clear privacy and legal boundary.
- Local diagnostics and recovery.

### 20.2 Useful later

- FTP/SFTP if customers need it.
- Site grabber for permitted content.
- OCR or media-specific processing is not part of a download manager core; evaluate as a separate product feature.
- Cloud queue sync.
- Mobile remote control.
- Enterprise policy management.
- HTTP/3/QUIC after measurement.
- Torrent/P2P after legal and security review.

### 20.3 Not needed for the current app

- Micro-frontends and Module Federation.
- Server-driven UI.
- CRDTs, vector clocks, or operational transformation.
- Event sourcing and distributed CQRS.
- Kafka, RabbitMQ, ESB, or a service mesh.
- GraphQL federation.
- Vector databases and time-series databases.
- Kubernetes, EKS/GKE, ArgoCD, or multi-cloud Terraform.
- Multi-region active-active replication.
- Mandatory OAuth/OIDC/MFA for a single-user offline utility.
- HSM integration for ordinary desktop downloads.
- GPU acceleration for HTTP transfer.
- Custom UDP/TCP framing.
- Anti-debugging as a security strategy.
- Mandatory online licensing.
- Default telemetry or cloud logging.

## 21. Roadmap

### Phase 0 — Technical proof

- Build a local fixture server.
- Prove HTTP range segmentation and single-stream fallback.
- Prove pause/resume across process restart.
- Prove safe atomic finalization and checksum verification.
- Prove memory remains bounded for large files.

**Exit:** deterministic transfer tests pass, including interruptions and malformed servers.

### Phase 1 — Usable desktop MVP

- Desktop shell and queue UI.
- Add URL, pause/resume/cancel/retry.
- Categories and destination selection.
- Global/task speed limits.
- Scheduler and shutdown action.
- Local SQLite state.

**Exit:** a user can manage normal HTTP/HTTPS downloads from one installed application.

### Phase 2 — IDM-class reliability

- Browser extensions.
- Better server compatibility and redirect policy.
- Resume validators and segment repair.
- Hash verification and antivirus adapter.
- History, diagnostics, and clean recovery.

**Exit:** measured compatibility and recovery are documented across fixture and real permitted download sources.

### Phase 3 — Release trust

- Code signing.
- Installer/update rollback.
- Clean-machine installation.
- Privacy review and terms.
- Browser-store submission if applicable.

**Exit:** public release has signed artifacts and a recovery-tested update path.

### Phase 4 — Optional enterprise edition

Only if customers require it:

- centralized policies;
- device management;
- enterprise identity;
- controlled update channels;
- support/audit export;
- optional queue synchronization.

This phase requires a new hosted architecture and threat model. It must not be smuggled into the desktop MVP through speculative infrastructure.

## 22. Downstream AI development instructions

Every implementation task must include:

1. exact files to change;
2. the command or interface contract being implemented;
3. error codes and retry behavior;
4. security and privacy constraints;
5. performance/resource budget;
6. unit and integration tests;
7. a manual acceptance step where the UI or installer changes;
8. a statement of what is intentionally out of scope.

Agents must stop for human review before:

- changing URL, redirect, cookie, or credential policy;
- adding a new browser permission;
- changing file deletion/overwrite behavior;
- adding an external service;
- changing update/signing/licensing behavior;
- altering database migrations;
- introducing telemetry;
- adding code that executes downloaded content.

## 23. Final recommendation to leadership

Invest first in transfer correctness and trust. The competitive advantage is not simply opening more connections; it is making downloads predictable under interruption, server incompatibility, disk pressure, browser handoff, and application restart.

The strongest first product is a fast, reliable, transparent local utility. A large enterprise cloud architecture would increase cost and attack surface without making the core download experience better. Add enterprise services only when customers pay for a real requirement that cannot be satisfied locally.

