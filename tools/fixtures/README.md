# SwiftFetch fixture server

A deterministic local HTTP server used to test the SwiftFetch transfer engine.
It implements the fixture contract in
`E:\CloneIdm\IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md` section 18.5.

This is a **test artifact**. It has no authentication and must only ever bind to
loopback. Never ship it with the product.

## Run

```bash
python tools/fixtures/server.py --host 127.0.0.1 --port 8734
```

## Self-test

```bash
python tools/fixtures/selftest.py
```

Starts the server on a free port, exercises every route, and exits non-zero on
any failure. Current status: **46/46 checks pass**.

## Routes

| Route | Behaviour |
| --- | --- |
| `/bytes/<size>` | 200, deterministic body of exactly `<size>` bytes, `Accept-Ranges: bytes`, stable ETag |
| `/range/<size>` | correct `206` + `Content-Range` for `Range: bytes=start-end`, open-ended, and suffix (`-N`) forms; `416` for unsatisfiable ranges |
| `/ignore-range/<size>` | always `200` with the full body, deliberately ignoring `Range`, advertises `Accept-Ranges: none` |
| `/bad-content-range` | `206` whose `Content-Range` does not match the request |
| `/short-body/<size>` | declares `Content-Length: <size>` but sends half, then closes |
| `/disconnect/<offset>` | sends `<offset>` bytes then closes the socket abruptly |
| `/status/<code>` | returns that HTTP status; `429` also sends `Retry-After: 1` |
| `/redirect/<count>` | `302` chain of `<count>` hops ending at `/bytes/1024` |
| `/redirect-loop` | redirects to itself forever |
| `/etag/<version>` | `ETag: "etag-<version>"`; changes when the version changes |
| `/slow/<delay-ms>` | sleeps before responding |
| `/checksum/<expected>` | fixed 8192-byte body; if `<expected>` is not the real sha256 the body is corrupted, so verification must fail |
| `/reset` | clears server-side counters |
| `/__stats` | JSON counters |
| `/__health` | liveness probe |

The sha256 of the `/checksum` body is printed when the server starts and is
asserted by the self-test.

## Notes for test authors

- The body content is `SWIFTFETCH-FIXTURE-DATA-CHUNK-0123456789\n` repeated and
  truncated, so a test can compute the expected bytes of any slice.
- `/range/<size>` supports only a single range. Multi-range requests get `416`.
- Connections are always closed after one response (`Connection: close`), which
  keeps the fixture simple and deterministic at the cost of connection reuse.
