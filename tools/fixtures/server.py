#!/usr/bin/env python3
"""SwiftFetch HTTP fixture server.

Deterministic local HTTP endpoints used by the Rust integration tests and for
manual debugging of the transfer engine. Implements the fixture contract in
``IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md`` section 18.5.

This server is a TEST artifact. It must never be shipped with the product and
must never be exposed on a non-loopback address in a real environment.

Routes
------
/bytes/<size>            normal single-stream body, honour HEAD
/range/<size>            correct 206 responses for Range requests
/ignore-range/<size>     always 200 with the full body, ignoring Range
/bad-content-range       206 with a malformed Content-Range header
/short-body/<size>       declares Content-Length <size> but sends less
/disconnect/<offset>     closes the connection after <offset> bytes
/status/<code>           returns the given HTTP status
/redirect/<count>        302 chain of <count> hops ending at /bytes/1024
/redirect-loop           redirects to itself forever
/etag/<version>          ETag and Last-Modified derived from <version>
/slow/<delay-ms>         delays headers and body by <delay-ms>
/checksum/<expected>     fixed 8192-byte body; <expected> is its sha256
/reset                   clears server-side counters (test helper)
/__stats                 JSON counters (test helper)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

CHUNK = b"SWIFTFETCH-FIXTURE-DATA-CHUNK-0123456789\n"

# Fixed body used by /checksum. Kept stable so an expected hash can be baked
# into a test.
CHECKSUM_BODY = (CHUNK * 400)[:8192]
CHECKSUM_SHA256 = hashlib.sha256(CHECKSUM_BODY).hexdigest()

STATS = {
    "requests": 0,
    "ranges_served": 0,
    "disconnects": 0,
}
STATS_LOCK = threading.Lock()


def make_body(size: int) -> bytes:
    """Return exactly ``size`` bytes of deterministic content."""
    if size <= 0:
        return b""
    out = bytearray()
    while len(out) < size:
        out += CHUNK
    return bytes(out[:size])


def parse_range(header: str | None, total: int) -> tuple[int, int] | None:
    """Parse a single ``bytes=start-end`` range. Returns None when invalid."""
    if not header:
        return None
    if "," in header:
        return None  # multi-range is intentionally unsupported
    m = header.strip()
    if not m.startswith("bytes="):
        return None
    spec = m[len("bytes=") :]
    if "-" not in spec:
        return None
    start_s, _, end_s = spec.partition("-")
    try:
        if start_s == "":
            # suffix range: last N bytes
            if end_s == "":
                return None
            n = int(end_s)
            if n <= 0 or total == 0:
                return None
            n = min(n, total)
            return (total - n, total - 1)
        start = int(start_s)
        end = int(end_s) if end_s else total - 1
    except ValueError:
        return None
    if start < 0 or start >= total:
        return None
    end = min(end, total - 1)
    if end < start:
        return None
    return (start, end)


class FixtureHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "SwiftFetchFixture/1.0"

    # ---- plumbing ---------------------------------------------------------

    def log_message(self, fmt: str, *args) -> None:  # keep test output clean
        if self.server.verbose:
            sys.stderr.write("[fixture] %s\n" % (fmt % args))

    def _count(self, key: str | None = None) -> None:
        with STATS_LOCK:
            STATS["requests"] += 1
            if key:
                STATS[key] += 1

    def _send(self, code: int, body: bytes = b"", headers: dict[str, str] | None = None) -> None:
        self.send_response(code)
        for k, v in (headers or {}).items():
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        if self.command != "HEAD" and body:
            self.wfile.write(body)

    def _not_found(self) -> None:
        self._send(404, b'{"error":"not found"}', {"Content-Type": "application/json"})

    # ---- routing ----------------------------------------------------------

    def do_HEAD(self) -> None:  # noqa: N802
        self._route(head_only=True)

    def do_GET(self) -> None:  # noqa: N802
        self._route(head_only=False)

    def _route(self, head_only: bool) -> None:
        parsed = urlparse(self.path)
        path = parsed.path
        qs = parse_qs(parsed.query)
        self._count()

        if path == "/__stats":
            body = json.dumps(STATS).encode()
            self._send(200, body, {"Content-Type": "application/json"})
            return
        if path == "/reset":
            with STATS_LOCK:
                STATS.update(requests=0, ranges_served=0, disconnects=0)
            self._send(200, b'{"ok":true}', {"Content-Type": "application/json"})
            return
        if path == "/__health":
            self._send(200, b"ok")
            return

        if path == "/status/200-error":
            self._send(200, b"sabr.malformed_config", {"Content-Type": "application/vnd.youtube-ump"})
        elif path == "/status/200-binary":
            self._send(200, make_body(64), {"Content-Type": "application/octet-stream"})
        elif path == "/manifest/hls.m3u8":
            self._send(200, b"#EXTM3U\n#EXT-X-ENDLIST\n#EXTINF:10,\n/bytes/128\n#EXTINF:10,\n/bytes/64\n", {"Content-Type": "application/vnd.apple.mpegurl"})
        elif path == "/manifest/hls-live.m3u8":
            self._send(200, b"#EXTM3U\n#EXTINF:10,\n/bytes/128\n", {"Content-Type": "application/vnd.apple.mpegurl"})
        elif path == "/manifest/hls-encrypted.m3u8":
            self._send(200, b"#EXTM3U\n#EXT-X-KEY:METHOD=AES-128\n#EXT-X-ENDLIST\n#EXTINF:10,\n/bytes/128\n", {"Content-Type": "application/vnd.apple.mpegurl"})
        elif path == "/manifest/hls-bad-segment.m3u8":
            self._send(200, b"#EXTM3U\n#EXT-X-ENDLIST\n#EXTINF:10,\n/status/500\n", {"Content-Type": "application/vnd.apple.mpegurl"})
        elif path == "/manifest/hls-slow.m3u8":
            self._send(200, b"#EXTM3U\n#EXT-X-ENDLIST\n#EXTINF:10,\n/slow/2000\n", {"Content-Type": "application/vnd.apple.mpegurl"})
        elif path == "/manifest/dash.mpd":
            self._send(200, b'<MPD><Representation id="test" bandwidth="1000"><SegmentTemplate initialization="/bytes/32" media="/bytes/64" startNumber="1"><SegmentTimeline><S d="4" r="1"/></SegmentTimeline></SegmentTemplate></Representation></MPD>', {"Content-Type": "application/dash+xml"})
        elif path.startswith("/bytes/"):
            self._handle_bytes(path.rsplit("/", 1)[-1], head_only)
        elif path.startswith("/range/"):
            self._handle_range(path.rsplit("/", 1)[-1], head_only)
        elif path.startswith("/ignore-range/"):
            self._handle_ignore_range(path.rsplit("/", 1)[-1], head_only)
        elif path == "/bad-content-range":
            self._handle_bad_content_range()
        elif path.startswith("/short-body/"):
            self._handle_short_body(path.rsplit("/", 1)[-1])
        elif path.startswith("/disconnect/"):
            self._handle_disconnect(path.rsplit("/", 1)[-1])
        elif path.startswith("/status/"):
            self._handle_status(path.rsplit("/", 1)[-1])
        elif path.startswith("/redirect-loop"):
            self._send(302, b"", {"Location": "/redirect-loop"})
        elif path.startswith("/redirect/"):
            self._handle_redirect(path.rsplit("/", 1)[-1])
        elif path.startswith("/etag/"):
            self._handle_etag(path.rsplit("/", 1)[-1], qs, head_only)
        elif path.startswith("/slow/"):
            self._handle_slow(path.rsplit("/", 1)[-1], head_only)
        elif path.startswith("/checksum/"):
            self._handle_checksum(path.rsplit("/", 1)[-1], head_only)
        else:
            self._not_found()

    # ---- handlers ---------------------------------------------------------

    def _handle_bytes(self, size_s: str, head_only: bool) -> None:
        try:
            size = int(size_s)
        except ValueError:
            return self._not_found()
        body = make_body(size)
        self._send(
            200,
            b"" if head_only else body,
            {
                "Content-Type": "application/octet-stream",
                "Accept-Ranges": "bytes",
                "ETag": '"bytes-%d"' % size,
                "Last-Modified": "Wed, 01 Jan 2025 00:00:00 GMT",
            },
        )

    def _handle_range(self, size_s: str, head_only: bool) -> None:
        try:
            size = int(size_s)
        except ValueError:
            return self._not_found()
        body = make_body(size)
        rng = parse_range(self.headers.get("Range"), size)
        if rng is None:
            # Unsatisfiable range.
            self.send_response(416)
            self.send_header("Content-Range", f"bytes */{size}")
            self.send_header("Content-Length", "0")
            self.send_header("Connection", "close")
            self.end_headers()
            return
        start, end = rng
        with STATS_LOCK:
            STATS["ranges_served"] += 1
        self.send_response(206)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.send_header("Content-Length", str(end - start + 1))
        self.send_header("ETag", '"range-%d"' % size)
        self.send_header("Connection", "close")
        self.end_headers()
        if not head_only:
            self.wfile.write(body[start : end + 1])

    def _handle_ignore_range(self, size_s: str, head_only: bool) -> None:
        try:
            size = int(size_s)
        except ValueError:
            return self._not_found()
        body = make_body(size)
        # Deliberately ignore the Range header: full 200 response.
        self._send(
            200,
            b"" if head_only else body,
            {"Content-Type": "application/octet-stream", "Accept-Ranges": "none"},
        )

    def _handle_bad_content_range(self) -> None:
        # A 206 whose Content-Range does not match the requested range.
        self.send_response(206)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Range", "bytes 0-0/999999999")
        self.send_header("Content-Length", "16")
        self.send_header("Connection", "close")
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(b"BADCONTENTRANGE!!")

    def _handle_short_body(self, size_s: str) -> None:
        try:
            size = int(size_s)
        except ValueError:
            return self._not_found()
        # Declare more than we send, then close.
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(size))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(make_body(size // 2))
        self.wfile.flush()
        self.close_connection = True

    def _handle_disconnect(self, offset_s: str) -> None:
        try:
            offset = int(offset_s)
        except ValueError:
            return self._not_found()
        payload = make_body(max(offset * 2, 1024))
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(payload[:offset])
        self.wfile.flush()
        with STATS_LOCK:
            STATS["disconnects"] += 1
        # Abrupt close without the remaining bytes.
        self.close_connection = True
        try:
            self.connection.shutdown(1)
        except OSError:
            pass

    def _handle_status(self, code_s: str) -> None:
        try:
            code = int(code_s)
        except ValueError:
            return self._not_found()
        body = json.dumps({"status": code}).encode()
        headers = {"Content-Type": "application/json"}
        if code == 429:
            headers["Retry-After"] = "1"
        self._send(code, body, headers)

    def _handle_redirect(self, count_s: str) -> None:
        try:
            count = int(count_s)
        except ValueError:
            return self._not_found()
        if count <= 0:
            self._send(302, b"", {"Location": "/bytes/1024"})
            return
        self._send(302, b"", {"Location": f"/redirect/{count - 1}"})

    def _handle_etag(self, version: str, qs: dict, head_only: bool) -> None:
        body = make_body(4096)
        etag = f'"etag-{version}"'
        self._send(
            200,
            b"" if head_only else body,
            {
                "Content-Type": "application/octet-stream",
                "Accept-Ranges": "bytes",
                "ETag": etag,
                "Last-Modified": f"Wed, 01 Jan 2025 00:0{abs(hash(version)) % 10}:00 GMT",
            },
        )

    def _handle_slow(self, delay_s: str, head_only: bool) -> None:
        try:
            delay = float(delay_s) / 1000.0
        except ValueError:
            return self._not_found()
        time.sleep(delay)
        body = make_body(2048)
        self._send(
            200,
            b"" if head_only else body,
            {"Content-Type": "application/octet-stream", "Accept-Ranges": "bytes"},
        )

    def _handle_checksum(self, expected: str, head_only: bool) -> None:
        body = CHECKSUM_BODY
        headers = {
            "Content-Type": "application/octet-stream",
            "Accept-Ranges": "bytes",
            "X-Fixture-Sha256": CHECKSUM_SHA256,
        }
        if expected and expected != CHECKSUM_SHA256:
            # Serve a corrupted body so checksum verification must fail.
            body = bytearray(body)
            body[0] ^= 0xFF
            body = bytes(body)
            headers["X-Fixture-Sha256"] = CHECKSUM_SHA256
        self._send(200, b"" if head_only else body, headers)


class FixtureServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, addr, handler, verbose=False):
        super().__init__(addr, handler)
        self.verbose = verbose


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="SwiftFetch HTTP fixture server")
    parser.add_argument("--host", default="127.0.0.1", help="bind address (default 127.0.0.1)")
    parser.add_argument("--port", type=int, default=8734, help="bind port (default 8734)")
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args(argv)

    if args.host not in ("127.0.0.1", "localhost", "::1"):
        print(
            "WARNING: binding to a non-loopback address exposes the fixture server "
            "to the network. It is a test artifact with no authentication.",
            file=sys.stderr,
        )

    server = FixtureServer((args.host, args.port), FixtureHandler, verbose=args.verbose)
    print(f"SwiftFetch fixture server listening on http://{args.host}:{args.port}")
    print(f"checksum body sha256 = {CHECKSUM_SHA256}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
