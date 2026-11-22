// ------------------------------------------------------------------ #
// GENERATED FILE — do not edit.
//
// Produced by tools/make-protocol-bundle.mjs from the matching ES module
// in this folder. Run `node browser-extension/tools/make-protocol-bundle.mjs`
// after changing the module. The classic form exists so Firefox Manifest V2
// can preload the protocol with `background.scripts`, without relying on a
// dynamic import() inside an event page. The module keeps its exports for
// Node tests and for the Chrome service worker.
// ------------------------------------------------------------------ #
(function () {
/**
 * SwiftFetch browser bridge — wire protocol and validators.
 *
 * This file is the single source of truth for the download_intent message
 * envelope and its validation rules. It is consumed by:
 *   - chrome/background.js  (Manifest V3 service worker)
 *   - firefox/background.js (Manifest V2 background page)
 *   - chrome/content.js     (optional client-side pre-check only)
 *   - chrome/popup.js       (optional client-side pre-check only)
 *   - native-host/host.py   (Python mirror of every rule below)
 *   - test/test_protocol.js (exhaustive test suite)
 *
 * The module is intentionally dependency-free and works as an ES module
 * (browsers, Node >= 14) and as a CommonJS module (Node require()).
 *
 * SECURITY POSTURE
 *   - Only http: and https: URLs and origins are accepted.
 *   - No passive traffic inspection: callers must only build an intent from an
 *     explicit user gesture or a user-configured per-site rule.
 *   - Cookies and authorization headers are never forwarded. The `cookies`
 *     array stays empty unless a caller explicitly opts in with
 *     `ctx.allowCookies === true`, which the shipped extension never does.
 *   - Expired and replayed intents are rejected.
 *   - suggested_name is untrusted text: it is reduced to a basename, stripped
 *     of separators, traversal, reserved device names and invalid characters.
 */
'use strict';

/* ------------------------------------------------------------------ *
 * Constants
 * ------------------------------------------------------------------ */

const PROTOCOL_VERSION = 1;
const PROTOCOL_V2 = 2;
const MESSAGE_TYPE = 'download_intent';

/**
 * Protocol v2 adds one optional `metadata` object. It never carries cookies,
 * headers or credentials: only bounded hints used by the confirmation UI.
 */
const METADATA_KINDS = Object.freeze(['ordinary', 'media']);
const METADATA_SITE_RULES = Object.freeze(['ask', 'allow', 'deny']);
const MAX_METADATA_LABEL = 128;
const MAX_MEDIA_VARIANTS = 32;
const MAX_SIZE_HINT = 100 * 1024 * 1024 * 1024; // 100 GiB, informational only

const MAX_URL_LENGTH = 2048;
const MAX_NAME_LENGTH = 200;
const MAX_CONTENT_TYPE_LENGTH = 255;
const MAX_TTL_MS = 5 * 60 * 1000; // an intent is only valid for 5 minutes
const DEFAULT_TTL_MS = 60 * 1000; // newly built intents expire after 1 minute
const REPLAY_WINDOW_MS = 5 * 60 * 1000; // duplicate request_id memory
const MAX_COOKIES = 16; // hard cap even if a caller opts in (extension never does)

const ALLOWED_SCHEMES = Object.freeze(['http:', 'https:']);

const INTENT_FIELDS = Object.freeze([
  'protocol',
  'type',
  'request_id',
  'url',
  'suggested_name',
  'page_origin',
  'content_type',
  'user_initiated',
  'cookies',
  'expires_at',
]);
const INTENT_FIELDS_V2 = Object.freeze([...INTENT_FIELDS, 'metadata']);

/** Fields that must never appear on the wire. */
const SENSITIVE_FIELDS = Object.freeze([
  'authorization',
  'proxy-authorization',
  'cookie',
  'cookies',
  'headers',
  'credentials',
]);

const UUID_V4_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

const CONTROL_CHAR_RE = /[\u0000-\u001f\u007f]/;
const WHITESPACE_RE = /\s/;
const MIME_RE = /^[A-Za-z0-9!#$&^_.+-]{1,127}\/[A-Za-z0-9!#$&^_.+-]{1,127}$/;
const ISO_8601_RE =
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2}(\.\d{1,3})?)?(Z|[+-]\d{2}:?\d{2})$/;

/** Windows reserved device names, with or without an extension. */
const RESERVED_NAMES = new Set(
  [
    'CON',
    'PRN',
    'AUX',
    'NUL',
    'CLOCK$',
    'COM1',
    'COM2',
    'COM3',
    'COM4',
    'COM5',
    'COM6',
    'COM7',
    'COM8',
    'COM9',
    'LPT1',
    'LPT2',
    'LPT3',
    'LPT4',
    'LPT5',
    'LPT6',
    'LPT7',
    'LPT8',
    'LPT9',
  ].map((name) => name.toUpperCase())
);

/* ------------------------------------------------------------------ *
 * Small helpers
 * ------------------------------------------------------------------ */

function succeed(value) {
  return { ok: true, value };
}

function fail(code, message) {
  return { ok: false, code, message };
}

function containsControlChars(value) {
  return typeof value === 'string' && CONTROL_CHAR_RE.test(value);
}

function containsWhitespace(value) {
  return typeof value === 'string' && WHITESPACE_RE.test(value);
}

function isValidUuidV4(value) {
  return typeof value === 'string' && UUID_V4_RE.test(value);
}

function parseTimestamp(value) {
  if (typeof value !== 'string' || value.length === 0) return null;
  if (CONTROL_CHAR_RE.test(value) || WHITESPACE_RE.test(value)) return null;
  if (!ISO_8601_RE.test(value)) return null;
  const ms = Date.parse(value);
  return Number.isFinite(ms) ? ms : null;
}

function newRequestId() {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  // RFC 4122 version 4 fallback for environments without WebCrypto.
  const bytes = new Uint8Array(16);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Math.floor(Math.random() * 256);
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join('-');
}

/* ------------------------------------------------------------------ *
 * Validators
 * ------------------------------------------------------------------ */

/**
 * Validate a download URL.
 * Returns { ok: true, value } or { ok: false, code, message }.
 */
function validateUrl(raw) {
  if (typeof raw !== 'string' || raw.length === 0) {
    return fail('invalid_url', 'url must be a non-empty string');
  }
  if (raw.length > MAX_URL_LENGTH) {
    return fail('url_too_long', `url must be at most ${MAX_URL_LENGTH} characters`);
  }
  if (containsControlChars(raw)) {
    return fail('url_control_chars', 'url must not contain control characters');
  }
  if (containsWhitespace(raw)) {
    return fail('url_control_chars', 'url must not contain whitespace');
  }
  let parsed;
  try {
    parsed = new URL(raw);
  } catch (error) {
    return fail('invalid_url', 'url is not a valid absolute URL');
  }
  if (!ALLOWED_SCHEMES.includes(parsed.protocol)) {
    return fail(
      'invalid_scheme',
      `scheme "${parsed.protocol}" is not allowed; only http: and https: are supported`
    );
  }
  if (!parsed.hostname) {
    return fail('invalid_url', 'url must include a host');
  }
  if (parsed.username || parsed.password) {
    return fail('url_credentials', 'url must not contain embedded credentials');
  }
  return succeed(raw);
}

/**
 * Validate a page origin. Must be a bare http/https origin: no path, no query,
 * no fragment, no credentials.
 */
function normalizeOrigin(raw) {
  if (typeof raw !== 'string' || raw.length === 0) {
    return fail('invalid_page_origin', 'page_origin must be a non-empty string');
  }
  if (containsControlChars(raw) || containsWhitespace(raw)) {
    return fail(
      'invalid_page_origin',
      'page_origin must not contain control characters or whitespace'
    );
  }
  if (raw.toLowerCase() === 'null') {
    return fail('invalid_page_origin', 'page_origin must not be "null"');
  }
  let parsed;
  try {
    parsed = new URL(raw);
  } catch (error) {
    return fail('invalid_page_origin', 'page_origin is not a valid URL');
  }
  if (!ALLOWED_SCHEMES.includes(parsed.protocol)) {
    return fail(
      'invalid_page_origin',
      `page_origin scheme "${parsed.protocol}" is not allowed`
    );
  }
  if (parsed.username || parsed.password) {
    return fail('invalid_page_origin', 'page_origin must not contain credentials');
  }
  if (parsed.pathname !== '/' || parsed.search !== '' || parsed.hash !== '') {
    return fail(
      'invalid_page_origin',
      'page_origin must be a bare origin with no path, query, or fragment'
    );
  }
  if (!parsed.hostname) {
    return fail('invalid_page_origin', 'page_origin must include a host');
  }
  const origin = parsed.origin;
  if (!origin || origin === 'null') {
    return fail('invalid_page_origin', 'page_origin could not be normalized');
  }
  return succeed(origin);
}

/**
 * Reduce an untrusted suggested filename to a safe basename.
 *
 * Rules:
 *   - must be a string, no control characters, no leading/trailing whitespace
 *   - no ".." path segment (traversal is rejected, never repaired)
 *   - reduced to the final path segment (separators can never escape)
 *   - no drive-letter or UNC form
 *   - Windows reserved device names are rejected
 *   - characters invalid on Windows are replaced
 *   - trailing dots/spaces are rejected
 *   - capped at MAX_NAME_LENGTH characters
 */
function sanitizeSuggestedName(raw) {
  if (typeof raw !== 'string') {
    return fail('invalid_suggested_name', 'suggested_name must be a string');
  }
  if (raw.length === 0) {
    return fail('invalid_suggested_name', 'suggested_name is empty');
  }
  if (raw.length > MAX_URL_LENGTH) {
    return fail('name_too_long', `suggested_name must be at most ${MAX_NAME_LENGTH} characters`);
  }
  if (containsControlChars(raw)) {
    return fail('name_control_chars', 'suggested_name must not contain control characters');
  }

  const trimmed = raw.trim();
  if (trimmed === '' || trimmed === '.' || trimmed === '..') {
    return fail('invalid_suggested_name', 'suggested_name is empty');
  }

  const segments = trimmed.split(/[\\/]+/);
  if (segments.some((segment) => segment === '..')) {
    return fail('path_traversal', 'suggested_name must not contain a ".." path segment');
  }

  const base = segments.filter((segment) => segment.length > 0).pop() || '';
  if (base === '') {
    return fail('invalid_suggested_name', 'suggested_name contains no usable filename');
  }
  if (/^[A-Za-z]:/.test(base)) {
    return fail('invalid_suggested_name', 'suggested_name must not be a drive path');
  }
  if (/[. ]$/.test(base)) {
    return fail(
      'invalid_suggested_name',
      'suggested_name must not end with a dot or space'
    );
  }

  let cleaned = base.replace(/[:\\/<>|?*"]/g, '_');
  cleaned = cleaned.replace(/^(.+?)[. ]+$/, '$1');
  cleaned = cleaned.replace(/^[. ]+/, '');
  if (cleaned === '') {
    return fail('invalid_suggested_name', 'suggested_name is empty after sanitising');
  }

  const upper = cleaned.toUpperCase();
  const stem = upper.split('.')[0];
  if (RESERVED_NAMES.has(upper) || RESERVED_NAMES.has(stem)) {
    return fail('reserved_name', `"${cleaned}" is a reserved Windows device name`);
  }

  if (cleaned.length > MAX_NAME_LENGTH) {
    cleaned = cleaned.slice(0, MAX_NAME_LENGTH).replace(/[. ]+$/, '');
  }
  if (cleaned === '') {
    return fail('invalid_suggested_name', 'suggested_name is empty after sanitising');
  }
  return succeed(cleaned);
}

/**
 * Validate a content type. Missing/empty values fall back to
 * application/octet-stream. Values must be a single RFC 7231 type/subtype.
 */
function normalizeContentType(raw) {
  if (raw === undefined || raw === null || raw === '') {
    return succeed('application/octet-stream');
  }
  if (typeof raw !== 'string') {
    return fail('invalid_content_type', 'content_type must be a string');
  }
  if (raw.length > MAX_CONTENT_TYPE_LENGTH) {
    return fail('invalid_content_type', 'content_type is too long');
  }
  if (containsControlChars(raw) || containsWhitespace(raw)) {
    return fail('invalid_content_type', 'content_type must not contain control characters');
  }
  const value = raw.trim().toLowerCase();
  if (!MIME_RE.test(value)) {
    return fail('invalid_content_type', 'content_type must be a type/subtype value');
  }
  return succeed(value);
}

/** True when the intent's expires_at is missing, unparseable, or in the past. */
function isExpiredIntent(intent, nowMs) {
  const now = typeof nowMs === 'number' ? nowMs : Date.now();
  if (!intent || typeof intent !== 'object') return true;
  const expiresMs = parseTimestamp(intent.expires_at);
  if (expiresMs === null) return true;
  return expiresMs <= now;
}

/** True when the intent expires later than the maximum allowed TTL. */
function isTooFarAheadIntent(intent, nowMs) {
  const now = typeof nowMs === 'number' ? nowMs : Date.now();
  if (!intent || typeof intent !== 'object') return false;
  const expiresMs = parseTimestamp(intent.expires_at);
  if (expiresMs === null) return false;
  return expiresMs - now > MAX_TTL_MS;
}

/**
 * Bounded replay guard. Remembers request_ids for `windowMs` and rejects
 * duplicates. Memory is bounded because entries are pruned on every check.
 */
function createReplayGuard(windowMs) {
  const window =
    typeof windowMs === 'number' && Number.isFinite(windowMs) && windowMs > 0
      ? windowMs
      : REPLAY_WINDOW_MS;
  const seen = new Map();

  function prune(now) {
    for (const [id, firstSeen] of seen) {
      if (now - firstSeen > window) seen.delete(id);
    }
  }

  return {
    /** Returns true when the id is new, false when it is a replay. */
    check(id, nowMs) {
      const now = typeof nowMs === 'number' ? nowMs : Date.now();
      prune(now);
      if (seen.has(id)) return false;
      seen.set(id, now);
      return true;
    },
    prune,
    clear() {
      seen.clear();
    },
    get size() {
      return seen.size;
    },
    get windowMs() {
      return window;
    },
  };
}

/* ------------------------------------------------------------------ *
 * Protocol v2 metadata (hints only, never credentials)
 * ------------------------------------------------------------------ */

/** Validate the bounded, non-sensitive v2 metadata object. */
function validateMetadata(raw) {
  if (raw === undefined || raw === null) return succeed(null);
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    return fail('invalid_metadata', 'metadata must be an object');
  }
  // Cross-realm messages (browser extension messaging) carry a different
  // Object.prototype, so use a realm-safe tag check instead of identity.
  if (Object.prototype.toString.call(raw) !== '[object Object]') {
    return fail('invalid_metadata', 'metadata must be a plain object');
  }
  for (const key of Object.keys(raw)) {
    if (!['size_hint', 'mime', 'kind', 'auto_accept', 'site_rule', 'media'].includes(key)) {
      return fail('invalid_metadata', `metadata field "${key}" is not allowed`);
    }
  }
  if (raw.size_hint !== undefined && raw.size_hint !== null) {
    if (typeof raw.size_hint !== 'number' || !Number.isFinite(raw.size_hint) || raw.size_hint < 0 || raw.size_hint > MAX_SIZE_HINT) {
      return fail('invalid_metadata', 'size_hint is out of range');
    }
  }
  if (raw.mime !== undefined && raw.mime !== null) {
    const mime = normalizeContentType(raw.mime);
    if (!mime.ok) return mime;
  }
  if (raw.kind !== undefined && raw.kind !== null && !METADATA_KINDS.includes(raw.kind)) {
    return fail('invalid_metadata', 'metadata.kind is not supported');
  }
  if (raw.auto_accept !== undefined && raw.auto_accept !== null && typeof raw.auto_accept !== 'boolean') {
    return fail('invalid_metadata', 'auto_accept must be a boolean');
  }
  if (raw.site_rule !== undefined && raw.site_rule !== null && !METADATA_SITE_RULES.includes(raw.site_rule)) {
    return fail('invalid_metadata', 'site_rule is not supported');
  }
  if (raw.media !== undefined && raw.media !== null) {
    const media = raw.media;
    if (typeof media !== 'object' || Array.isArray(media)) {
      return fail('invalid_metadata', 'media must be an object');
    }
    const url = validateUrl(media.url);
    if (!url.ok) return url;
    if (media.mime !== undefined && media.mime !== null) {
      const mime = normalizeContentType(media.mime);
      if (!mime.ok) return mime;
    }
    if (media.label !== undefined && media.label !== null) {
      if (typeof media.label !== 'string' || media.label.length > MAX_METADATA_LABEL || CONTROL_CHAR_RE.test(media.label)) {
        return fail('invalid_metadata', 'media.label is invalid');
      }
    }
    for (const dim of ['width', 'height']) {
      if (media[dim] !== undefined && media[dim] !== null) {
        const value = media[dim];
        if (typeof value !== 'number' || !Number.isInteger(value) || value < 0 || value > 10000) {
          return fail('invalid_metadata', `media.${dim} is out of range`);
        }
      }
    }
    if (media.variants !== undefined && media.variants !== null) {
      if (!Array.isArray(media.variants) || media.variants.length > MAX_MEDIA_VARIANTS) {
        return fail('invalid_metadata', 'too many media variants');
      }
      for (const variant of media.variants) {
        const variantUrl = validateUrl(variant && variant.url);
        if (!variantUrl.ok) return variantUrl;
        if (variant.label !== undefined && (typeof variant.label !== 'string' || variant.label.length > MAX_METADATA_LABEL)) {
          return fail('invalid_metadata', 'variant label is invalid');
        }
      }
    }
  }
  return succeed(raw);
}

/* ------------------------------------------------------------------ *
 * Intent validation
 * ------------------------------------------------------------------ */

/**
 * Reject anything that is not a plain object, and any field that is not part
 * of the protocol. This is what structurally prevents authorization headers,
 * cookies-by-default, or arbitrary extra JSON from ever being forwarded.
 */
function assertNoSecrets(payload) {
  if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) {
    return fail('invalid_message', 'message must be a JSON object');
  }
  const prototype = Object.getPrototypeOf(payload);
  if (prototype !== Object.prototype && prototype !== null) {
    return fail('invalid_message', 'message must be a plain JSON object');
  }
  for (const key of Object.keys(payload)) {
    const lower = key.toLowerCase().replace(/_/g, '-');
    // "cookies" is a protocol field with its own strict rule (it is forced to
    // an empty array); it must not be mistaken for an injected secret here.
    if (SENSITIVE_FIELDS.includes(lower) && !INTENT_FIELDS.includes(key)) {
      return fail('sensitive_field', `field "${key}" must never be sent`);
    }
  }
  return succeed(true);
}

function hasUnexpectedFields(payload) {
  const allowed = payload.protocol === PROTOCOL_V2 ? INTENT_FIELDS_V2 : INTENT_FIELDS;
  for (const key of Object.keys(payload)) {
    if (!allowed.includes(key)) return key;
  }
  return null;
}

/**
 * Validate a raw download_intent message.
 *
 * ctx:
 *   now           - epoch ms to validate against (defaults to Date.now())
 *   allowCookies  - opt in to forwarding a non-empty cookies array (the
 *                   shipped extension never sets this)
 *   replayGuard   - object from createReplayGuard(), or a Set of seen ids
 *
 * Returns { ok: true, intent } with a canonical, normalised intent, or
 * { ok: false, code, message }.
 */
function validateIntent(raw, ctx) {
  const context = ctx || {};
  const now = typeof context.now === 'number' ? context.now : Date.now();

  const secrets = assertNoSecrets(raw);
  if (!secrets.ok) return secrets;

  const unexpected = hasUnexpectedFields(raw);
  if (unexpected !== null) {
    return fail(
      'unexpected_field',
      `field "${unexpected}" is not part of the download_intent protocol`
    );
  }

  if (raw.protocol !== PROTOCOL_VERSION && raw.protocol !== PROTOCOL_V2) {
    return fail('invalid_protocol', `protocol must be ${PROTOCOL_VERSION} or ${PROTOCOL_V2}`);
  }
  if (raw.type !== MESSAGE_TYPE) {
    return fail('invalid_type', `type must be "${MESSAGE_TYPE}"`);
  }
  if (!isValidUuidV4(raw.request_id)) {
    return fail('invalid_request_id', 'request_id must be a UUID version 4 string');
  }

  const url = validateUrl(raw.url);
  if (!url.ok) return url;

  const origin = normalizeOrigin(raw.page_origin);
  if (!origin.ok) return origin;

  let suggestedName = null;
  if (raw.suggested_name !== undefined && raw.suggested_name !== null && raw.suggested_name !== '') {
    const name = sanitizeSuggestedName(raw.suggested_name);
    if (!name.ok) return name;
    suggestedName = name.value;
  }

  const contentType = normalizeContentType(raw.content_type);
  if (!contentType.ok) return contentType;

  if (raw.user_initiated !== true) {
    return fail(
      'missing_user_initiated',
      'user_initiated must be exactly true; passive capture is not permitted'
    );
  }

  if (!Array.isArray(raw.cookies)) {
    return fail('invalid_cookies', 'cookies must be an array');
  }
  if (raw.cookies.length > 0 && context.allowCookies !== true) {
    return fail(
      'cookies_not_allowed',
      'cookies are never forwarded unless the user explicitly opts in per download'
    );
  }
  if (raw.cookies.length > MAX_COOKIES) {
    return fail('cookies_not_allowed', `at most ${MAX_COOKIES} cookies may be sent`);
  }

  const expiresMs = parseTimestamp(raw.expires_at);
  if (expiresMs === null) {
    return fail('invalid_expires_at', 'expires_at must be an ISO 8601 timestamp');
  }
  if (expiresMs <= now) {
    return fail('expired_intent', 'the download intent has expired');
  }
  if (expiresMs - now > MAX_TTL_MS) {
    return fail(
      'expired_intent',
      `expires_at must be within ${MAX_TTL_MS / 1000} seconds of now`
    );
  }

  if (context.replayGuard) {
    const accepted =
      typeof context.replayGuard.check === 'function'
        ? context.replayGuard.check(raw.request_id, now)
        : !context.replayGuard.has(raw.request_id);
    if (!accepted) {
      return fail('replay_intent', 'this request_id has already been used');
    }
    if (typeof context.replayGuard.check !== 'function') {
      context.replayGuard.add(raw.request_id);
    }
  }

  let metadata = null;
  if (raw.protocol === PROTOCOL_V2) {
    const checked = validateMetadata(raw.metadata);
    if (!checked.ok) return checked;
    metadata = checked.value;
  }

  const canonical = {
    protocol: raw.protocol,
    type: MESSAGE_TYPE,
    request_id: raw.request_id,
    url: raw.url,
    suggested_name: suggestedName,
    page_origin: origin.value,
    content_type: contentType.value,
    user_initiated: true,
    cookies: context.allowCookies === true ? raw.cookies.slice(0, MAX_COOKIES) : [],
    expires_at: new Date(expiresMs).toISOString(),
  };
  if (metadata !== null) canonical.metadata = metadata;
  return succeed(canonical);
}

/**
 * Build and validate a download_intent in one step, so that every message the
 * extension emits has already passed the full validation rules.
 */
function buildIntent(input, ctx) {
  const context = ctx || {};
  const now = typeof context.now === 'number' ? context.now : Date.now();
  const requestedTtl =
    typeof input.ttlMs === 'number' && Number.isFinite(input.ttlMs) && input.ttlMs > 0
      ? Math.min(input.ttlMs, MAX_TTL_MS)
      : DEFAULT_TTL_MS;

  const useV2 = context.protocolV2 === true || (input.metadata !== undefined && input.metadata !== null);
  const raw = {
    protocol: useV2 ? PROTOCOL_V2 : PROTOCOL_VERSION,
    type: MESSAGE_TYPE,
    request_id: input.requestId || newRequestId(),
    url: input.url,
    suggested_name: input.suggestedName === undefined ? null : input.suggestedName,
    page_origin: input.pageOrigin,
    content_type: input.contentType || 'application/octet-stream',
    // Callers must only invoke buildIntent from a gesture handler. When the
    // flag is omitted we assume a gesture happened; passing `false` explicitly
    // still produces a message that validation rejects.
    user_initiated: input.userInitiated === undefined ? true : input.userInitiated === true,
    cookies: Array.isArray(input.cookies) ? input.cookies : [],
    expires_at: new Date(now + requestedTtl).toISOString(),
  };
  if (useV2) raw.metadata = input.metadata || {};
  return validateIntent(raw, context);
}

/* ------------------------------------------------------------------ *
 * Logging / redaction
 * ------------------------------------------------------------------ */

/**
 * Return a log-safe summary. The URL path and query are never included because
 * they frequently carry session tokens; only scheme and host are kept.
 */
function redactForLog(intent) {
  const safe = {
    protocol: intent.protocol,
    type: intent.type,
    request_id: intent.request_id,
    page_origin: intent.page_origin,
    content_type: intent.content_type,
    user_initiated: intent.user_initiated,
    expires_at: intent.expires_at,
    suggested_name: intent.suggested_name,
    url_host: null,
    url_path_redacted: true,
    cookies_redacted: true,
  };
  try {
    const parsed = new URL(intent.url);
    safe.url_host = `${parsed.protocol}//${parsed.host}`;
  } catch (error) {
    safe.url_host = '<unparseable>';
  }
  return safe;
}

/* ------------------------------------------------------------------ *
 * Exports
 * ------------------------------------------------------------------ */

const api = {
  PROTOCOL_VERSION,
  PROTOCOL_V2,
  MESSAGE_TYPE,
  MAX_URL_LENGTH,
  MAX_NAME_LENGTH,
  MAX_CONTENT_TYPE_LENGTH,
  MAX_TTL_MS,
  DEFAULT_TTL_MS,
  REPLAY_WINDOW_MS,
  MAX_COOKIES,
  ALLOWED_SCHEMES,
  INTENT_FIELDS,
  INTENT_FIELDS_V2,
  SENSITIVE_FIELDS,
  UUID_V4_RE,
  ISO_8601_RE,
  RESERVED_NAMES,
  succeed,
  fail,
  containsControlChars,
  containsWhitespace,
  isValidUuidV4,
  parseTimestamp,
  newRequestId,
  validateUrl,
  normalizeOrigin,
  sanitizeSuggestedName,
  normalizeContentType,
  isExpiredIntent,
  isTooFarAheadIntent,
  createReplayGuard,
  assertNoSecrets,
  validateMetadata,
  validateIntent,
  buildIntent,
  redactForLog,
};

if (typeof globalThis !== 'undefined') {
  globalThis.SwiftFetchProtocol = api;
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = api;
}

/* Classic-script export: available as a global in a non-module context. */
globalThis.SwiftFetchProtocol = api;

})();
