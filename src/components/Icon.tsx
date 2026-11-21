import type { CSSProperties } from "react";

const paths = {
  bolt: "M13 2 4 14h7l-1 8 10-13h-7l1-7Z",
  dashboard: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  queue: "M8 6h13 M8 12h13 M8 18h13 M3 6h.01 M3 12h.01 M3 18h.01",
  history: "M3 11a9 9 0 1 1 2.5 7 M3 4v7h7 M12 7v5l3 2",
  clock: "M12 8v4l3 2 M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
  folder: "M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10H3V7Z",
  settings: "m9 3-1 3-3 1v3l-2 2 2 2v3l3 1 1 3h6l1-3 3-1v-3l2-2-2-2V7l-3-1-1-3H9Z M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z",
  activity: "M2 12h4l3-8 6 16 3-8h4",
  plus: "M12 5v14 M5 12h14",
  download: "M12 3v12 m-5-5 5 5 5-5 M5 16v5h14v-5",
  pause: "M8 5v14 M16 5v14",
  play: "m7 4 14 8-14 8V4Z",
  cancel: "M7 7l10 10 M17 7 7 17 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z",
  retry: "M21 10a9 9 0 1 0-2.5 8 M21 3v7h-7",
  check: "m5 12 4 4L19 6",
  shield: "m12 3 9 4v6c0 4-5 7-9 9-4-2-9-5-9-9V7l9-4Z m-4 10 3 3 5-6",
  close: "m6 6 12 12 M18 6 6 18",
  chevron: "m9 5 7 7-7 7",
  search: "M21 21l-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z",
  file: "M14 2H4v20h16V8l-6-6Z M14 2v6h6 M8 13h8 M8 17h5",
  warning: "m12 3 10 18H2L12 3Z M12 9v5 M12 17h.01",
  info: "M12 11v6 M12 7h.01 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z",
  copy: "M9 9h12v12H9z M15 9V3H3v12h6",
  trash: "M3 6h18 M9 6V3h6v3 M5 6l1 15h12l1-15 M10 10v7 M14 10v7",
  edit: "m16 3 5 5-12 12-6 1 1-6L16 3Z M13 6l5 5",
  offline: "m3 3 18 18 M2 8a16 16 0 0 1 4-2 M10 5a16 16 0 0 1 12 3 M5 12a11 11 0 0 1 5-3 M15 10a11 11 0 0 1 4 2 M8 16a6 6 0 0 1 8 0 M12 20h.01",
} satisfies Record<string, string>;

export type IconName = keyof typeof paths;

export function Icon({ name, size = 20, className, style }: { name: IconName; size?: number; className?: string; style?: CSSProperties }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false" className={className} style={style}><path d={paths[name]} /></svg>;
}
