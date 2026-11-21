/**
 * Localization-ready typed dictionaries. Only keys that exist here are
 * translated; every other string keeps its English source text. Copy that is
 * not yet translated must not be described as fully localized.
 */
import { createContext, useContext, type ReactNode } from "react";

export type Locale = "en" | "bn";
export const SUPPORTED_LOCALES: readonly Locale[] = ["en", "bn"];

export function normalizeLocale(value: string | null | undefined): Locale {
    return value === "bn" ? "bn" : "en";
}

export function applyDocumentLocale(locale: Locale): void {
    if (typeof document === "undefined") return;
    document.documentElement.lang = locale;
    document.documentElement.dir = "ltr"; // Both supported locales are LTR.
}

export const en = {
    "nav.workspace": "WORKSPACE",
    "nav.dashboard": "Overview",
    "nav.dashboard.desc": "Your downloads at a glance.",
    "nav.queue": "Download queue",
    "nav.queue.desc": "Prioritize, pause, and pick up where you left off.",
    "nav.history": "History",
    "nav.history.desc": "A record of your finished transfers.",
    "nav.scheduler": "Scheduler",
    "nav.scheduler.desc": "Make time work for your downloads.",
    "nav.categories": "Categories",
    "nav.categories.desc": "Keep every download in its place.",
    "nav.settings": "Settings",
    "nav.settings.desc": "A workspace that works your way.",
    "nav.diagnostics": "Diagnostics",
    "nav.diagnostics.desc": "Inspect local, redacted core information.",
    "app.add": "Add download",
    "app.add.shortcut": "Add download (Ctrl+N)",
    "app.refresh": "Refresh downloads",
    "app.skip": "Skip to main content",
    "app.active": "active",
    "app.offline": "Network offline",
    "app.coreConnected": "Core connected",
    "app.connecting": "Connecting to core",
    "app.preview": "Browser preview · no core",
    "app.localTitle": "Local by design",
    "app.localBody": "No account. No cloud upload.",
    "app.footer": "Your files stay on your device",
    "banner.offline.title": "Network appears offline.",
    "banner.offline.body": "Keep managing local tasks and settings. Transfer status comes from the core; reconnect before resuming internet downloads.",
    "banner.events": "Live event registration failed. The UI refreshes its snapshot every 10 seconds while visible; progress may be delayed.",
    "banner.recovery": "Recovery completed by the core",
    "status.queued": "Queued",
    "status.starting": "Starting",
    "status.downloading": "Downloading",
    "status.paused": "Paused",
    "status.retry_wait": "Waiting to retry",
    "status.verifying": "Verifying",
    "status.scanning": "Scanning",
    "status.completed": "Completed",
    "status.failed": "Failed",
    "status.cancelled": "Cancelled",
    "action.start": "Start",
    "action.pause": "Pause",
    "action.resume": "Resume",
    "action.cancel": "Cancel",
    "action.retry": "Retry",
    "action.verify": "Verify checksum",
    "action.open_file": "Open file",
    "action.open_folder": "Show in folder",
    "action.remove_history": "Remove from history",
    "action.set_priority": "Set priority",
    "action.overwrite": "Replace existing file",
    "action.rename": "Use a new name",
    "action.skip": "Skip this file",
    "dialog.add": "Add download",
    "dialog.paste": "Paste URL",
    "dialog.details": "Task details",
    "dialog.settings": "Settings",
    "dialog.cancel": "Cancel",
    "dialog.close": "Close",
    "dialog.closeDialog": "Close dialog",
    "dialog.done": "Done",
    "dialog.save": "Save settings",
    "dialog.goBack": "Go back",
    "notice.added": "Download added and confirmed with the desktop core.",
    "power.pending": "A system shutdown is pending",
    "power.cancel": "Cancel shutdown",
    "update.title": "About & updates",
    "update.notConfigured": "Updates are not configured in this build. No update endpoint or signing key is available, so no update check runs.",
    "update.check": "Check for updates",
    "update.install": "Install update",
    "update.rollback": "Roll back update",
} as const;

export type MessageKey = keyof typeof en;

export const bn: Record<MessageKey, string> = {
    "nav.workspace": "কার্যক্ষেত্র",
    "nav.dashboard": "সারাংশ",
    "nav.dashboard.desc": "আপনার ডাউনলোড এক নজরে।",
    "nav.queue": "ডাউনলোড সারি",
    "nav.queue.desc": "অগ্রাধিকার দিন, থামান এবং আগের জায়গা থেকে শুরু করুন।",
    "nav.history": "ইতিহাস",
    "nav.history.desc": "সমাপ্ত ডাউনলোডের নথি।",
    "nav.scheduler": "সময়সূচী",
    "nav.scheduler.desc": "আপনার ডাউনলোডের জন্য সময় নির্ধারণ করুন।",
    "nav.categories": "বিভাগ",
    "nav.categories.desc": "প্রতিটি ডাউনলোড তার নিজস্ব জায়গায় রাখুন।",
    "nav.settings": "সেটিংস",
    "nav.settings.desc": "আপনার পছন্দ অনুযায়ী কাজের ক্ষেত্র।",
    "nav.diagnostics": "রোগ নির্ণয়",
    "nav.diagnostics.desc": "স্থানীয়, গোপনীয়তা-রক্ষিত কোর তথ্য দেখুন।",
    "app.add": "ডাউনলোড যোগ করুন",
    "app.add.shortcut": "ডাউনলোড যোগ করুন (Ctrl+N)",
    "app.refresh": "ডাউনলোড রিফ্রেশ করুন",
    "app.skip": "মুখ্য বিষয়বস্তুতে যান",
    "app.active": "সক্রিয়",
    "app.offline": "নেটওয়ার্ক অফলাইন",
    "app.coreConnected": "কোর সংযুক্ত",
    "app.connecting": "কোরের সাথে সংযোগ হচ্ছে",
    "app.preview": "ব্রাউজার প্রিভিউ · কোর নেই",
    "app.localTitle": "স্থানীয় নকশা",
    "app.localBody": "কোনো অ্যাকাউন্ট নেই। ক্লাউডে আপলোড নেই।",
    "app.footer": "আপনার ফাইল আপনার ডিভাইসেই থাকে",
    "banner.offline.title": "নেটওয়ার্ক অফলাইন বলে মনে হচ্ছে।",
    "banner.offline.body": "স্থানীয় কাজ এবং সেটিংস পরিচালনা করুন। স্থানান্তরের অবস্থা কোর থেকে আসে; ইন্টারনেট ডাউনলোড আবার শুরু করার আগে সংযোগ করুন।",
    "banner.events": "লাইভ ইভেন্ট নিবন্ধন ব্যর্থ হয়েছে। ফাইল দৃশ্যমান থাকা অবস্থায় UI প্রতি ১০ সেকেন্ডে রিফ্রেশ হয়; অগ্রগতি বিলম্বিত হতে পারে।",
    "banner.recovery": "কোর দ্বারা পুনরুদ্ধার সম্পন্ন",
    "status.queued": "সারিতে আছে",
    "status.starting": "শুরু হচ্ছে",
    "status.downloading": "ডাউনলোড হচ্ছে",
    "status.paused": "থামানো হয়েছে",
    "status.retry_wait": "আবার চেষ্টার অপেক্ষায়",
    "status.verifying": "যাচাই হচ্ছে",
    "status.scanning": "স্ক্যান হচ্ছে",
    "status.completed": "সম্পন্ন",
    "status.failed": "ব্যর্থ",
    "status.cancelled": "বাতিল",
    "action.start": "শুরু",
    "action.pause": "থামান",
    "action.resume": "আবার শুরু",
    "action.cancel": "বাতিল",
    "action.retry": "আবার চেষ্টা",
    "action.verify": "চেকসাম যাচাই",
    "action.open_file": "ফাইল খুলুন",
    "action.open_folder": "ফোল্ডার দেখান",
    "action.remove_history": "ইতিহাস থেকে সরান",
    "action.set_priority": "অগ্রাধিকার নির্ধারণ",
    "action.overwrite": "বিদ্যমান ফাইল প্রতিস্থাপন",
    "action.rename": "নতুন নাম ব্যবহার",
    "action.skip": "এই ফাইল এড়িয়ে যান",
    "dialog.add": "ডাউনলোড যোগ করুন",
    "dialog.paste": "URL পেস্ট করুন",
    "dialog.details": "কাজের বিশদ",
    "dialog.settings": "সেটিংস",
    "dialog.cancel": "বাতিল",
    "dialog.close": "বন্ধ করুন",
    "dialog.closeDialog": "ডায়ালগ বন্ধ করুন",
    "dialog.done": "সম্পন্ন",
    "dialog.save": "সেটিংস সংরক্ষণ",
    "dialog.goBack": "ফিরে যান",
    "notice.added": "ডাউনলোড যোগ করা হয়েছে এবং ডেস্কটপ কোর দ্বারা নিশ্চিত।",
    "power.pending": "সিস্টেম বন্ধ করার অনুরোধ মুলতুবি আছে",
    "power.cancel": "সিস্টেম বন্ধ বাতিল করুন",
    "update.title": "সম্পর্কে ও আপডেট",
    "update.notConfigured": "এই বিল্ডে আপডেট কনফিগার করা নেই। কোনো আপডেট এন্ডপয়েন্ট বা স্বাক্ষর কী নেই, তাই কোনো আপডেট চেক চলে না।",
    "update.check": "আপডেট পরীক্ষা করুন",
    "update.install": "আপডেট ইনস্টল করুন",
    "update.rollback": "আপডেট ফিরিয়ে আনুন",
};

export function translate(locale: Locale | string | null | undefined, key: MessageKey): string {
    const table = normalizeLocale(locale) === "bn" ? bn : en;
    return table[key] ?? en[key];
}

const LocaleContext = createContext<Locale>("en");

export function LocaleProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
    return <LocaleContext.Provider value={locale}>{children}</LocaleContext.Provider>;
}

export function useT(): (key: MessageKey) => string {
    const locale = useContext(LocaleContext);
    return (key: MessageKey) => translate(locale, key);
}
