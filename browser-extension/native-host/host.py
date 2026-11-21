#!/usr/bin/env python3
"""SwiftFetch native messaging host.

Reads length-prefixed JSON messages on stdin (Chrome/Firefox native messaging
framing: 4-byte little-endian length, then that many bytes of UTF-8 JSON),
validates each message with the same rules as shared/protocol.js, forwards it
to the SwiftFetch desktop app over loopback HTTP with an HMAC-SHA256 header,
and writes the app's reply back to stdout in the same framing.

Security rules mirrored from shared/protocol.js (keep the two in sync):
  * only http: and https: URLs and origins
  * URL length <= 2048, no control characters or whitespace
  * suggested_name reduced to a safe basename, traversal rejected
  * cookies and authorization headers are never forwarded
  * user_initiated must be true
  * expired and replayed intents are rejected
  * only loopback app URLs are allowed

Configuration (environment variables):
  SWIFTFETCH_SHARED_SECRET  required. HMAC key shared with the SwiftFetch app.
  SWIFTFETCH_APP_URL        default http://127.0.0.1:27642/bridge/intent
  SWIFTFETCH_HOST_LOG       default %LOCALAPPDATA%\\SwiftFetch\\native-host.log
  SWIFTFETCH_MAX_BODY       default 1048576 (1 MiB inbound message cap)

Logs never contain cookies, authorization headers, or the full URL path/query.
"""

import calendar
import hashlib
import hmac
import json
import os
import re
import struct
import sys
import time
import urllib.error
import urllib.request
from urllib.parse import urlsplit

PROTOCOL_VERSION = 1
MESSAGE_TYPE = "download_intent"

MAX_URL_LENGTH = 2048
MAX_NAME_LENGTH = 200
MAX_CONTENT_TYPE_LENGTH = 255
MAX_TTL_MS = 5 * 60 * 1000
REPLAY_WINDOW_MS = 5 * 60 * 1000
MAX_COOKIES = 16
MAX_REPLAY_ENTRIES = 5000
MAX_MESSAGE_BYTES = 1 << 20
HTTP_TIMEOUT_SECONDS = 10
ALLOWED_SCHEMES = ("http", "https")
LOOPBACK_HOSTS = ("127.0.0.1", "localhost", "::1")

INTENT_FIELDS = frozenset(
    [
        "protocol",
        "type",
        "request_id",
        "url",
        "suggested_name",
        "page_origin",
        "content_type",
        "user_initiated",
        "cookies",
        "expires_at",
    ]
)

SENSITIVE_FIELDS = frozenset(
    [
        "authorization",
        "proxy-authorization",
        "cookie",
        "cookies",
        "headers",
        "credentials",
    ]
)

UUID_V4_RE = re.compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
    re.IGNORECASE,
)
CONTROL_CHAR_RE = re.compile(r"[\u0000-\u001f\u007f]")
WHITESPACE_RE = re.compile(r"\s")
MIME_RE = re.compile(r"^[A-Za-z0-9!#$&^_.+-]{1,127}/[A-Za-z0-9!#$&^_.+-]{1,127}$")
ISO_8601_RE = re.compile(
    r"^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?"
    r"(Z|[+-]\d{2}:?\d{2})$"
)

RESERVED_NAMES = frozenset(
    name.upper()
    for name in [
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "CLOCK$",
        "COM1",
        "COM2",
        "COM3",
        "COM4",
        "COM5",
        "COM6",
        "COM7",
        "COM8",
        "COM9",
        "LPT1",
        "LPT2",
        "LPT3",
        "LPT4",
        "LPT5",
        "LPT6",
        "LPT7",
        "LPT8",
        "LPT9",
    ]
)


# --------------------------------------------------------------------- #
# Helpers
# --------------------------------------------------------------------- #


def fail(code, message):
    return {"ok": False, "code": code, "message": message}


def ok(value):
    return {"ok": True, "value": value}


def is_plain_object(value):
    return isinstance(value, dict)


def contains_control_chars(value):
    return isinstance(value, str) and CONTROL_CHAR_RE.search(value) is not None


def contains_whitespace(value):
    return isinstance(value, str) and WHITESPACE_RE.search(value) is not None


def is_uuid_v4(value):
    return isinstance(value, str) and UUID_V4_RE.match(value) is not None


def parse_timestamp(value):
    """Return epoch milliseconds for an ISO-8601 timestamp, or None."""
    if not isinstance(value, str) or not value:
        return None
    if contains_control_chars(value) or contains_whitespace(value):
        return None
    match = ISO_8601_RE.match(value)
    if match is None:
        return None
    year, month, day, hour, minute = (int(match.group(i)) for i in range(1, 6))
    second = int(match.group(6) or 0)
    fraction = match.group(7) or ""
    millis = int((fraction + "000")[:3]) if fraction else 0
    zone = match.group(8)
    try:
        calendar.timegm((year, month, day, hour, minute, second, 0, 0, 0))
    except (ValueError, OverflowError):
        return None
    if not (1 <= month <= 12 and 1 <= day <= 31 and hour <= 23 and minute <= 59 and second <= 60):
        return None
    if zone == "Z":
        offset = 0
    else:
        sign = 1 if zone[0] == "+" else -1
        body = zone[1:].replace(":", "")
        offset = sign * (int(body[:2]) * 3600 + int(body[2:4]) * 60)
    return (calendar.timegm((year, month, day, hour, minute, second, 0, 0, 0)) - offset) * 1000 + millis


# --------------------------------------------------------------------- #
# Validators (mirror of shared/protocol.js)
# --------------------------------------------------------------------- #


def validate_url(raw):
    if not isinstance(raw, str) or not raw:
        return fail("invalid_url", "url must be a non-empty string")
    if len(raw) > MAX_URL_LENGTH:
        return fail("url_too_long", "url must be at most %d characters" % MAX_URL_LENGTH)
    if contains_control_chars(raw):
        return fail("url_control_chars", "url must not contain control characters")
    if contains_whitespace(raw):
        return fail("url_control_chars", "url must not contain whitespace")
    try:
        parts = urlsplit(raw)
    except ValueError:
        return fail("invalid_url", "url is not a valid absolute URL")
    if parts.scheme.lower() not in ALLOWED_SCHEMES:
        return fail(
            "invalid_scheme",
            'scheme "%s" is not allowed; only http: and https: are supported' % parts.scheme,
        )
    if not parts.hostname:
        return fail("invalid_url", "url must include a host")
    if parts.username or parts.password:
        return fail("url_credentials", "url must not contain embedded credentials")
    return ok(raw)


def normalize_origin(raw):
    if not isinstance(raw, str) or not raw:
        return fail("invalid_page_origin", "page_origin must be a non-empty string")
    if contains_control_chars(raw) or contains_whitespace(raw):
        return fail(
            "invalid_page_origin",
            "page_origin must not contain control characters or whitespace",
        )
    if raw.lower() == "null":
        return fail("invalid_page_origin", 'page_origin must not be "null"')
    try:
        parts = urlsplit(raw)
        port = parts.port
    except ValueError:
        return fail("invalid_page_origin", "page_origin is not a valid URL")
    if parts.scheme.lower() not in ALLOWED_SCHEMES:
        return fail(
            "invalid_page_origin",
            'page_origin scheme "%s" is not allowed' % parts.scheme,
        )
    if parts.username or parts.password:
        return fail("invalid_page_origin", "page_origin must not contain credentials")
    if parts.path not in ("", "/") or parts.query or parts.fragment:
        return fail(
            "invalid_page_origin",
            "page_origin must be a bare origin with no path, query, or fragment",
        )
    if not parts.hostname:
        return fail("invalid_page_origin", "page_origin must include a host")
    scheme = parts.scheme.lower()
    host = parts.hostname.lower()
    if ":" in host:
        host = "[%s]" % host
    netloc = host
    if port is not None and not (
        (scheme == "http" and port == 80) or (scheme == "https" and port == 443)
    ):
        netloc = "%s:%d" % (host, port)
    origin = "%s://%s" % (scheme, netloc)
    if not origin or origin == "null":
        return fail("invalid_page_origin", "page_origin could not be normalized")
    return ok(origin)


def sanitize_suggested_name(raw):
    if not isinstance(raw, str):
        return fail("invalid_suggested_name", "suggested_name must be a string")
    if not raw:
        return fail("invalid_suggested_name", "suggested_name is empty")
    if len(raw) > MAX_URL_LENGTH:
        return fail("name_too_long", "suggested_name must be at most %d characters" % MAX_NAME_LENGTH)
    if contains_control_chars(raw):
        return fail("name_control_chars", "suggested_name must not contain control characters")

    trimmed = raw.strip()
    if trimmed in ("", ".", ".."):
        return fail("invalid_suggested_name", "suggested_name is empty")

    segments = re.split(r"[\\/]+", trimmed)
    if ".." in segments:
        return fail("path_traversal", 'suggested_name must not contain a ".." path segment')

    base = ""
    for segment in reversed(segments):
        if segment:
            base = segment
            break
    if not base:
        return fail("invalid_suggested_name", "suggested_name contains no usable filename")
    if re.match(r"^[A-Za-z]:", base):
        return fail("invalid_suggested_name", "suggested_name must not be a drive path")
    if base.endswith(".") or base.endswith(" "):
        return fail("invalid_suggested_name", "suggested_name must not end with a dot or space")

    cleaned = re.sub(r'[:\\/<>|?*"]', "_", base)
    cleaned = re.sub(r"^(.+?)[. ]+$", r"\1", cleaned)
    cleaned = re.sub(r"^[. ]+", "", cleaned)
    if not cleaned:
        return fail("invalid_suggested_name", "suggested_name is empty after sanitising")

    upper = cleaned.upper()
    stem = upper.split(".")[0]
    if upper in RESERVED_NAMES or stem in RESERVED_NAMES:
        return fail("reserved_name", '"%s" is a reserved Windows device name' % cleaned)

    if len(cleaned) > MAX_NAME_LENGTH:
        cleaned = re.sub(r"[. ]+$", "", cleaned[:MAX_NAME_LENGTH])
    if not cleaned:
        return fail("invalid_suggested_name", "suggested_name is empty after sanitising")
    return ok(cleaned)


def normalize_content_type(raw):
    if raw is None or raw == "":
        return ok("application/octet-stream")
    if not isinstance(raw, str):
        return fail("invalid_content_type", "content_type must be a string")
    if len(raw) > MAX_CONTENT_TYPE_LENGTH:
        return fail("invalid_content_type", "content_type is too long")
    if contains_control_chars(raw) or contains_whitespace(raw):
        return fail("invalid_content_type", "content_type must not contain control characters")
    value = raw.strip().lower()
    if MIME_RE.match(value) is None:
        return fail("invalid_content_type", "content_type must be a type/subtype value")
    return ok(value)


def assert_no_secrets(payload):
    if not is_plain_object(payload):
        return fail("invalid_message", "message must be a JSON object")
    for key in payload:
        lowered = key.lower().replace("_", "-")
        # "cookies" is a protocol field with its own strict rule (it is forced
        # to an empty array); it must not be mistaken for an injected secret.
        if lowered in SENSITIVE_FIELDS and key not in INTENT_FIELDS:
            return fail("sensitive_field", 'field "%s" must never be sent' % key)
    return ok(True)


def has_unexpected_fields(payload):
    for key in payload:
        if key not in INTENT_FIELDS:
            return key
    return None


def validate_intent(raw, ctx):
    """Validate a download_intent message. ctx: now, allow_cookies, replay_seen."""
    context = ctx or {}
    now = context.get("now")
    if not isinstance(now, (int, float)):
        now = int(time.time() * 1000)

    secrets = assert_no_secrets(raw)
    if not secrets["ok"]:
        return secrets

    unexpected = has_unexpected_fields(raw)
    if unexpected is not None:
        return fail(
            "unexpected_field",
            'field "%s" is not part of the download_intent protocol' % unexpected,
        )

    if raw.get("protocol") != PROTOCOL_VERSION:
        return fail("invalid_protocol", "protocol must be %d" % PROTOCOL_VERSION)
    if raw.get("type") != MESSAGE_TYPE:
        return fail("invalid_type", 'type must be "%s"' % MESSAGE_TYPE)
    if not is_uuid_v4(raw.get("request_id")):
        return fail("invalid_request_id", "request_id must be a UUID version 4 string")

    url = validate_url(raw.get("url"))
    if not url["ok"]:
        return url

    origin = normalize_origin(raw.get("page_origin"))
    if not origin["ok"]:
        return origin

    suggested_name = None
    candidate = raw.get("suggested_name")
    if candidate is not None and candidate != "":
        name = sanitize_suggested_name(candidate)
        if not name["ok"]:
            return name
        suggested_name = name["value"]

    content_type = normalize_content_type(raw.get("content_type"))
    if not content_type["ok"]:
        return content_type

    if raw.get("user_initiated") is not True:
        return fail(
            "missing_user_initiated",
            "user_initiated must be exactly true; passive capture is not permitted",
        )

    cookies = raw.get("cookies")
    if not isinstance(cookies, list):
        return fail("invalid_cookies", "cookies must be an array")
    if cookies and context.get("allow_cookies") is not True:
        return fail(
            "cookies_not_allowed",
            "cookies are never forwarded unless the user explicitly opts in per download",
        )
    if len(cookies) > MAX_COOKIES:
        return fail("cookies_not_allowed", "at most %d cookies may be sent" % MAX_COOKIES)

    expires_ms = parse_timestamp(raw.get("expires_at"))
    if expires_ms is None:
        return fail("invalid_expires_at", "expires_at must be an ISO 8601 timestamp")
    if expires_ms <= now:
        return fail("expired_intent", "the download intent has expired")
    if expires_ms - now > MAX_TTL_MS:
        return fail(
            "expired_intent",
            "expires_at must be within %d seconds of now" % (MAX_TTL_MS // 1000),
        )

    seen = context.get("replay_seen")
    if isinstance(seen, dict):
        seen.pop("__none__", None)
        if raw["request_id"] in seen:
            return fail("replay_intent", "this request_id has already been used")

    return ok(
        {
            "protocol": PROTOCOL_VERSION,
            "type": MESSAGE_TYPE,
            "request_id": raw["request_id"],
            "url": raw["url"],
            "suggested_name": suggested_name,
            "page_origin": origin["value"],
            "content_type": content_type["value"],
            "user_initiated": True,
            "cookies": cookies[:MAX_COOKIES] if context.get("allow_cookies") is True else [],
            "expires_at": raw["expires_at"],
        }
    )


def redact_for_log(intent):
    safe = {
        "protocol": intent.get("protocol"),
        "type": intent.get("type"),
        "request_id": intent.get("request_id"),
        "page_origin": intent.get("page_origin"),
        "content_type": intent.get("content_type"),
        "user_initiated": intent.get("user_initiated"),
        "expires_at": intent.get("expires_at"),
        "suggested_name": intent.get("suggested_name"),
        "url_host": None,
        "url_path_redacted": True,
        "cookies_redacted": True,
    }
    try:
        parts = urlsplit(intent.get("url", ""))
        safe["url_host"] = "%s://%s" % (parts.scheme, parts.netloc)
    except ValueError:
        safe["url_host"] = "<unparseable>"
    return safe


# --------------------------------------------------------------------- #
# Logging (never stdout: stdout is the protocol channel)
# --------------------------------------------------------------------- #

_LOG_PATH = None
_log_warned = False


def log_path():
    global _LOG_PATH
    if _LOG_PATH is None:
        configured = os.environ.get("SWIFTFETCH_HOST_LOG")
        if configured:
            _LOG_PATH = os.path.expandvars(configured)
        else:
            local_appdata = os.environ.get("LOCALAPPDATA", os.path.expanduser("~"))
            _LOG_PATH = os.path.join(local_appdata, "SwiftFetch", "native-host.log")
    return _LOG_PATH


def log(message):
    global _log_warned
    try:
        directory = os.path.dirname(log_path())
        if directory:
            os.makedirs(directory, exist_ok=True)
        with open(log_path(), "a", encoding="utf-8") as handle:
            handle.write("%.3f %s\n" % (time.time(), message))
    except OSError:
        if not _log_warned:
            _log_warned = True
            print("[swiftfetch] cannot write log file", file=sys.stderr)


# --------------------------------------------------------------------- #
# Native messaging framing
# --------------------------------------------------------------------- #


def read_message(stream):
    header = stream.read(4)
    if not header:
        return None
    if len(header) < 4:
        return fail("truncated_header", "native messaging header is truncated")
    (length,) = struct.unpack("<I", header)
    if length > MAX_MESSAGE_BYTES:
        return fail("message_too_large", "message exceeds the %d byte limit" % MAX_MESSAGE_BYTES)
    body = stream.read(length)
    if body is None or len(body) < length:
        return fail("truncated_body", "native messaging body is truncated")
    try:
        return ok(json.loads(body.decode("utf-8")))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return fail("invalid_json", "message is not valid UTF-8 JSON")


def write_message(stream, payload):
    body = json.dumps(payload, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    stream.write(struct.pack("<I", len(body)))
    stream.write(body)
    stream.flush()


# --------------------------------------------------------------------- #
# Loopback bridge
# --------------------------------------------------------------------- #


def app_url():
    return os.environ.get("SWIFTFETCH_APP_URL", "http://127.0.0.1:27642/bridge/intent")


def assert_loopback(url):
    try:
        parts = urlsplit(url)
        port = parts.port
    except ValueError:
        return fail("bad_app_url", "SWIFTFETCH_APP_URL is not a valid URL")
    if parts.scheme.lower() != "http":
        return fail("bad_app_url", "SWIFTFETCH_APP_URL must use http")
    if (parts.hostname or "").lower() not in LOOPBACK_HOSTS:
        return fail(
            "bad_app_url",
            "SWIFTFETCH_APP_URL must point at the loopback interface",
        )
    if port is None or not (1 <= port <= 65535):
        return fail("bad_app_url", "SWIFTFETCH_APP_URL must include a valid port")
    return ok(True)


def forward(intent, secret):
    """POST the intent to the SwiftFetch app. Returns a reply dict."""
    target = app_url()
    target_check = assert_loopback(target)
    if not target_check["ok"]:
        return {
            "ok": False,
            "error": {
                "code": target_check["code"],
                "message": target_check["message"],
                "retryable": True,
            },
        }

    body = json.dumps(intent, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    timestamp = str(int(time.time()))
    request_id = intent.get("request_id", "")
    signature = hmac.new(
        secret.encode("utf-8"),
        ("%s.%s." % (timestamp, request_id)).encode("utf-8") + body,
        hashlib.sha256,
    ).hexdigest()

    request = urllib.request.Request(
        target,
        data=body,
        method="POST",
        headers={
            "Content-Type": "application/json",
            "X-SwiftFetch-Request-Id": request_id,
            "X-SwiftFetch-Timestamp": timestamp,
            "X-SwiftFetch-Signature": signature,
            "X-SwiftFetch-Host-Version": "1",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            raw = response.read(MAX_MESSAGE_BYTES + 1)
            status = response.status
    except urllib.error.HTTPError as error:
        raw = error.read(MAX_MESSAGE_BYTES + 1)
        status = error.code
    except (urllib.error.URLError, OSError) as error:
        log("app unreachable: %s" % type(error).__name__)
        return {
            "ok": False,
            "error": {
                "code": "app_unreachable",
                "message": "SwiftFetch is not running or the bridge is disabled.",
                "retryable": True,
            },
        }

    if len(raw) > MAX_MESSAGE_BYTES:
        return {
            "ok": False,
            "error": {
                "code": "app_reply_too_large",
                "message": "SwiftFetch returned an oversized reply.",
                "retryable": True,
            },
        }
    try:
        parsed = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        parsed = None

    if not isinstance(parsed, dict):
        return {
            "ok": False,
            "error": {
                "code": "app_bad_response",
                "message": "SwiftFetch returned an invalid response.",
                "retryable": True,
            },
        }
    if status < 200 or status >= 300:
        nested = parsed.get("error") if isinstance(parsed.get("error"), dict) else {}
        return {
            "ok": False,
            "error": {
                "code": nested.get("code", "app_error"),
                "message": nested.get("message", "SwiftFetch rejected the download."),
                "retryable": nested.get("retryable", True) is not False,
            },
        }
    if parsed.get("ok") is True:
        return {
            "ok": True,
            "task_id": parsed.get("task_id") if isinstance(parsed.get("task_id"), str) else None,
        }
    nested = parsed.get("error") if isinstance(parsed.get("error"), dict) else {}
    return {
        "ok": False,
        "error": {
            "code": nested.get("code", "app_error"),
            "message": nested.get("message", "SwiftFetch rejected the download."),
            "retryable": nested.get("retryable", True) is not False,
        },
    }


# --------------------------------------------------------------------- #
# Main loop
# --------------------------------------------------------------------- #


def prune_replay(seen, now):
    stale = [key for key, first_seen in seen.items() if now - first_seen > REPLAY_WINDOW_MS]
    for key in stale:
        del seen[key]
    # Bound the cache so a long-running machine cannot grow it without limit.
    if len(seen) > MAX_REPLAY_ENTRIES:
        overflow = len(seen) - MAX_REPLAY_ENTRIES
        for key in sorted(seen, key=lambda item: seen[item])[:overflow]:
            del seen[key]


def replay_file():
    configured = os.environ.get("SWIFTFETCH_REPLAY_FILE")
    if configured:
        return os.path.expandvars(configured)
    local_appdata = os.environ.get("LOCALAPPDATA", os.path.expanduser("~"))
    return os.path.join(local_appdata, "SwiftFetch", "native-host-replay.json")


def load_replay_state(path):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            data = json.load(handle)
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def save_replay_state(path, seen):
    try:
        directory = os.path.dirname(path)
        if directory:
            os.makedirs(directory, exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(seen, handle, separators=(",", ":"))
    except OSError:
        # Fail open: the app is the final replay authority.
        log("could not persist replay state")


def handle_message(raw, secret, replay_seen, replay_path):
    now = int(time.time() * 1000)
    verdict = validate_intent(
        raw,
        {
            "now": now,
            "allow_cookies": False,
            "replay_seen": replay_seen,
        },
    )
    if not verdict["ok"]:
        return {
            "ok": False,
            "error": {
                "code": verdict["code"],
                "message": verdict["message"],
                "retryable": verdict["code"] in ("expired_intent", "replay_intent"),
            },
        }

    intent = verdict["value"]
    replay_seen[intent["request_id"]] = now
    prune_replay(replay_seen, now)
    save_replay_state(replay_path, replay_seen)

    log("forwarding intent " + json.dumps(redact_for_log(intent), sort_keys=True))
    reply = forward(intent, secret)
    log("app reply code=" + str(reply.get("error", {}).get("code", "ok")))
    return reply


def main():
    secret = os.environ.get("SWIFTFETCH_SHARED_SECRET")
    if not secret:
        print("[swiftfetch] SWIFTFETCH_SHARED_SECRET is not set", file=sys.stderr)
        return 2

    stdin = getattr(sys.stdin, "buffer", sys.stdin)
    stdout = getattr(sys.stdout, "buffer", sys.stdout)
    replay_path = replay_file()
    replay_seen = load_replay_state(replay_path)

    while True:
        message = read_message(stdin)
        if message is None:
            return 0
        if not message["ok"]:
            log("bad inbound message: " + message["code"])
            write_message(
                stdout,
                {
                    "ok": False,
                    "error": {
                        "code": message["code"],
                        "message": message["message"],
                        "retryable": False,
                    },
                },
            )
            continue
        try:
            reply = handle_message(message["value"], secret, replay_seen, replay_path)
        except Exception as error:  # never crash the browser's native host
            log("host exception: " + type(error).__name__)
            reply = {
                "ok": False,
                "error": {
                    "code": "host_error",
                    "message": "The SwiftFetch native host failed to process this message.",
                    "retryable": True,
                },
            }
        write_message(stdout, reply)


if __name__ == "__main__":
    sys.exit(main())
