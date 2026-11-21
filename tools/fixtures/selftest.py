#!/usr/bin/env python3
"""Self-test for the SwiftFetch fixture server.

Starts tools/fixtures/server.py on a free loopback port, exercises every route
against the contract in the architecture specification (section 18.5), and
prints a pass/fail summary. Exits non-zero if anything fails.

Usage:
    python tools/fixtures/selftest.py
"""

from __future__ import annotations

import hashlib
import http.client
import json
import os
import socket
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "server.py")

RESULTS: list[tuple[bool, str, str]] = []


def free_port() -> int:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def request(
    port: int,
    path: str,
    method: str = "GET",
    headers: dict[str, str] | None = None,
    follow_redirects: bool = False,
    max_redirects: int = 10,
) -> tuple[int, dict[str, str], bytes]:
    """Raw HTTP request. Returns (status, headers, body). Raises on dead socket."""
    hops = 0
    current = path
    while True:
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=15)
        try:
            conn.request(method, current, headers=headers or {})
            resp = conn.getresponse()
            body = resp.read()
            hdrs = {k.lower(): v for k, v in resp.getheaders()}
            status = resp.status
        finally:
            conn.close()
        if follow_redirects and status in (301, 302, 303, 307, 308):
            location = hdrs.get("location")
            if location and hops < max_redirects:
                hops += 1
                current = location
                continue
        return status, hdrs, body


def check(name: str, cond: bool, detail: str = "") -> None:
    RESULTS.append((cond, name, detail))
    mark = "PASS" if cond else "FAIL"
    print(f"  [{mark}] {name}" + (f" — {detail}" if detail and not cond else ""))


def expect_incomplete(name: str, fn) -> None:
    """Assert that fn() raises (connection dropped mid-body)."""
    try:
        fn()
    except Exception as exc:  # noqa: BLE001 - any failure means the cut happened
        check(name, True, f"raised {type(exc).__name__}")
        return
    check(name, False, "connection was not interrupted")


def main() -> int:
    port = free_port()
    print(f"Starting fixture server on 127.0.0.1:{port}")
    proc = subprocess.Popen(
        [sys.executable, SERVER, "--host", "127.0.0.1", "--port", str(port)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        deadline = time.time() + 20
        while time.time() < deadline:
            try:
                status, _, _ = request(port, "/__health")
                if status == 200:
                    break
            except Exception:  # noqa: BLE001
                time.sleep(0.2)
        else:
            print("FIXTURE SERVER DID NOT START")
            return 2

        # --- normal body -------------------------------------------------
        status, hdrs, body = request(port, "/bytes/1024")
        check("GET /bytes/1024 returns 200", status == 200, f"status={status}")
        check("/bytes/1024 body length", len(body) == 1024, f"len={len(body)}")
        check("/bytes/1024 advertises ranges", hdrs.get("accept-ranges") == "bytes")

        # HEAD should not carry a body
        status, hdrs, body = request(port, "/bytes/1024", method="HEAD")
        check("HEAD /bytes/1024 returns 200 and no body", status == 200 and body == b"")

        # --- range responses --------------------------------------------
        status, hdrs, body = request(port, "/range/4096", headers={"Range": "bytes=0-0"})
        check("Range bytes=0-0 returns 206", status == 206, f"status={status}")
        check(
            "Range bytes=0-0 Content-Range",
            hdrs.get("content-range") == "bytes 0-0/4096",
            f"got {hdrs.get('content-range')!r}",
        )
        check("Range bytes=0-0 body length 1", len(body) == 1, f"len={len(body)}")

        status, hdrs, body = request(port, "/range/4096", headers={"Range": "bytes=100-199"})
        check("Range bytes=100-199 returns 206", status == 206)
        check("Range bytes=100-199 length 100", len(body) == 100, f"len={len(body)}")
        check(
            "Range bytes=100-199 content matches",
            body == (b"SWIFTFETCH-FIXTURE-DATA-CHUNK-0123456789\n" * 400)[:4096][100:200],
        )

        status, hdrs, body = request(port, "/range/4096", headers={"Range": "bytes=900-"})
        check("open-ended Range returns 206", status == 206)
        check("open-ended Range length", len(body) == 4096 - 900, f"len={len(body)}")

        status, hdrs, body = request(port, "/range/4096", headers={"Range": "bytes=-100"})
        check("suffix Range returns 206", status == 206)
        check("suffix Range length 100", len(body) == 100, f"len={len(body)}")

        status, hdrs, body = request(port, "/range/4096", headers={"Range": "bytes=5000-6000"})
        check("out-of-range returns 416", status == 416, f"status={status}")
        check(
            "416 carries Content-Range */size",
            hdrs.get("content-range") == "bytes */4096",
            f"got {hdrs.get('content-range')!r}",
        )

        # --- range ignored ----------------------------------------------
        status, hdrs, body = request(port, "/ignore-range/1024", headers={"Range": "bytes=0-0"})
        check("ignored Range still returns 200", status == 200, f"status={status}")
        check("ignored Range returns full body", len(body) == 1024, f"len={len(body)}")
        check("ignored Range advertises Accept-Ranges: none", hdrs.get("accept-ranges") == "none")

        # --- malformed Content-Range ------------------------------------
        status, hdrs, body = request(port, "/bad-content-range", headers={"Range": "bytes=0-15"})
        check("bad-content-range returns 206", status == 206, f"status={status}")
        check(
            "bad-content-range Content-Range is wrong",
            hdrs.get("content-range") != "bytes 0-15/16",
            f"got {hdrs.get('content-range')!r}",
        )

        # --- status codes -----------------------------------------------
        for code in (401, 403, 404, 408, 410, 429, 500, 502, 503, 504):
            status, hdrs, _ = request(port, f"/status/{code}")
            check(f"GET /status/{code} returns {code}", status == code, f"status={status}")
        _, hdrs, _ = request(port, "/status/429")
        check("429 sends Retry-After", hdrs.get("retry-after") == "1", f"got {hdrs.get('retry-after')!r}")

        # --- redirects ---------------------------------------------------
        status, hdrs, body = request(port, "/redirect/3", follow_redirects=True)
        check("redirect chain resolves to 200", status == 200, f"status={status}")
        check("redirect chain serves 1024 bytes", len(body) == 1024, f"len={len(body)}")

        status, _, _ = request(port, "/redirect-loop", follow_redirects=True)
        check("redirect loop is bounded", status in (302, 500, 508, 400), f"status={status}")

        # --- validators --------------------------------------------------
        _, hdrs1, _ = request(port, "/etag/v1")
        _, hdrs2, _ = request(port, "/etag/v2")
        check("etag v1 present", hdrs1.get("etag") == '"etag-v1"', f"got {hdrs1.get('etag')!r}")
        check("etag changes with version", hdrs1.get("etag") != hdrs2.get("etag"))

        # --- checksum ----------------------------------------------------
        expected = hashlib.sha256(
            (b"SWIFTFETCH-FIXTURE-DATA-CHUNK-0123456789\n" * 400)[:8192]
        ).hexdigest()
        status, hdrs, body = request(port, f"/checksum/{expected}")
        check("checksum route returns 200", status == 200)
        check("checksum body is 8192 bytes", len(body) == 8192, f"len={len(body)}")
        check(
            "checksum body matches declared hash",
            hashlib.sha256(body).hexdigest() == expected,
        )
        _, hdrs, bad = request(port, "/checksum/0" * 1)
        check(
            "checksum route corrupts body when expected hash differs",
            hashlib.sha256(bad).hexdigest() != expected,
        )

        # --- interruptions ----------------------------------------------
        expect_incomplete(
            "disconnect route cuts the connection",
            lambda: request(port, "/disconnect/100"),
        )
        expect_incomplete(
            "short-body route under-delivers",
            lambda: request(port, "/short-body/2048"),
        )

        # --- sl ----------------------------------------------------------
        start = time.time()
        request(port, "/slow/300")
        elapsed = time.time() - start
        check("slow route honours the delay", elapsed >= 0.25, f"elapsed={elapsed:.3f}s")

        # --- stats ---------------------------------------------------------
        status, _, body = request(port, "/__stats")
        stats = json.loads(body) if status == 200 else {}
        check("stats endpoint reports requests", stats.get("requests", 0) > 10, f"stats={stats}")
        check("stats endpoint reports ranges", stats.get("ranges_served", 0) > 0)

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()

    failed = [r for r in RESULTS if not r[0]]
    print()
    print(f"{len(RESULTS) - len(failed)}/{len(RESULTS)} checks passed")
    if failed:
        print("\nFAILURES:")
        for _, name, detail in failed:
            print(f"  - {name}: {detail}")
        return 1
    print("ALL FIXTURE CHECKS PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
