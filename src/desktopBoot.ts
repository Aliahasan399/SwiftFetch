import type { Settings } from "./types";

/** Only a usable task catalog and settings document can acknowledge an update. */
export async function bootDesktop(
  tasks: () => Promise<void>, settings: () => Promise<Settings>,
  apply: (settings: Settings) => void, ready: () => Promise<unknown>,
  current: () => boolean = () => true,
): Promise<void> {
  const [, confirmed] = await Promise.all([tasks(), settings()]);
  if (!current()) return;
  apply(confirmed);
  // Phase 3: privacy-safe timing marks (labels only), read by the startup probe.
  performance.mark("swiftfetch:frontend-ready");
  await ready();
  performance.mark("swiftfetch:health-acknowledged");
}
