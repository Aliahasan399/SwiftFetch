/**
 * Build a shareable diagnostics preview without uploading anything.
 * Sensitive fields are removed before the JSON is shown or copied.
 */
const SENSITIVE_KEYS = new Set([
  "url", "original_url", "final_url", "destination_path", "temporary_path",
  "headers", "headers_json", "cookies", "cookie", "authorization", "token",
  "api_key", "secret", "password", "database_path", "private_key", "signature",
]);

function redact(value: unknown, key = ""): unknown {
  if (key && SENSITIVE_KEYS.has(key.toLowerCase().replace(/[^a-z_]/g, ""))) {
    return "[redacted]";
  }
  if (Array.isArray(value)) return value.map((item) => redact(item));
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([name, item]) => [name, redact(item, name)]),
    );
  }
  return value;
}

export function redactDiagnosticsBundle(data: unknown): string {
  return JSON.stringify(redact(data), null, 2);
}
