import { api, deviceKey, errorMessage, type ActivityEntry, type Device, type Environment, type Platform } from "./api";

const SETTINGS_KEY = "mobile-dev-util:settings";
const BOOT_GRACE_MS = 180_000;
const MAX_ACTIVITY = 200;

export interface Settings {
  platform: Platform;
  outputDir: string;
  androidSdk: string;
  coldBoot: boolean;
  recentUrls: string[];
  selected: Record<Platform, string | null>;
}

export interface Toast {
  id: number;
  kind: "success" | "error" | "info";
  title: string;
  message?: string;
  path?: string;
}

const defaults: Settings = {
  platform: "android",
  outputDir: "",
  androidSdk: "",
  coldBoot: false,
  recentUrls: [],
  selected: { android: null, ios: null },
};

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    return raw ? { ...defaults, ...JSON.parse(raw) } : { ...defaults };
  } catch {
    return { ...defaults };
  }
}

class AppStore {
  settings = $state<Settings>(loadSettings());
  env = $state<Environment | null>(null);
  devices = $state<Record<Platform, Device[]>>({ android: [], ios: [] });
  errors = $state<Record<Platform, string | null>>({ android: null, ios: null });
  loaded = $state<Record<Platform, boolean>>({ android: false, ios: false });
  refreshing = $state(false);
  /** deviceKey -> label of the action currently running. */
  busy = $state<Record<string, string>>({});
  /** deviceKey -> timestamp of a boot we requested but haven't seen yet. */
  bootRequested = $state<Record<string, number>>({});
  /** deviceKey -> timestamp the recording started. */
  recordingSince = $state<Record<string, number>>({});
  activity = $state<ActivityEntry[]>([]);
  toasts = $state<Toast[]>([]);
  now = $state(Date.now());

  #toastId = 0;
  #inflight: Partial<Record<Platform, Promise<void>>> = {};

  get platform() {
    return this.settings.platform;
  }

  set platform(p: Platform) {
    this.settings.platform = p;
  }

  get currentDevices() {
    return this.devices[this.platform];
  }

  get selected(): Device | null {
    const id = this.settings.selected[this.platform];
    return this.currentDevices.find((d) => d.id === id) ?? null;
  }

  select(device: Device) {
    this.settings.selected[device.platform] = device.id;
  }

  persist() {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify($state.snapshot(this.settings)));
  }

  /** Device state including optimistic "booting" after a boot request. */
  effectiveState(d: Device) {
    const requested = this.bootRequested[deviceKey(d)];
    if (d.state === "shutdown" && requested && this.now - requested < BOOT_GRACE_MS) {
      return "booting" as const;
    }
    return d.state;
  }

  async init() {
    this.env = await api.getEnvironment();
    if (this.settings.androidSdk && this.settings.androidSdk !== this.env.sdkOverride) {
      this.env = await api.setAndroidSdk(this.settings.androidSdk);
    }
    await this.refresh(this.platform);
  }

  refresh(platform: Platform = this.platform): Promise<void> {
    // Coalesce overlapping polls for the same platform.
    const existing = this.#inflight[platform];
    if (existing) return existing;

    const run = (async () => {
      if (platform === this.platform) this.refreshing = true;
      try {
        const list = await api.listDevices(platform);
        this.devices[platform] = list;
        this.errors[platform] = null;
        for (const d of list) {
          const key = deviceKey(d);
          if (d.state !== "shutdown") delete this.bootRequested[key];
          if (!d.recording) delete this.recordingSince[key];
          else if (!this.recordingSince[key]) this.recordingSince[key] = Date.now();
        }
        if (!list.some((d) => d.id === this.settings.selected[platform])) {
          const fallback = list.find((d) => d.state === "booted") ?? list[0];
          this.settings.selected[platform] = fallback?.id ?? null;
        }
      } catch (e) {
        this.devices[platform] = [];
        this.errors[platform] = errorMessage(e);
      } finally {
        this.loaded[platform] = true;
        this.refreshing = false;
        delete this.#inflight[platform];
      }
    })();
    this.#inflight[platform] = run;
    return run;
  }

  /** Runs a device action with busy state, toasts and a follow-up refresh. */
  async run<T>(
    device: Device,
    label: string,
    action: () => Promise<T>,
    onSuccess?: (result: T) => Omit<Toast, "id"> | void,
  ): Promise<T | undefined> {
    const key = deviceKey(device);
    this.busy[key] = label;
    try {
      const result = await action();
      const toast = onSuccess?.(result);
      if (toast) this.toast(toast);
      return result;
    } catch (e) {
      this.toast({ kind: "error", title: `${label} failed`, message: errorMessage(e) });
      return undefined;
    } finally {
      delete this.busy[key];
      void this.refresh(device.platform);
    }
  }

  async boot(device: Device, coldBoot = this.settings.coldBoot) {
    const key = deviceKey(device);
    this.bootRequested[key] = Date.now();
    const ok = await this.run(device, "Boot", () => api.boot(device, coldBoot).then(() => true));
    if (!ok) delete this.bootRequested[key];
  }

  shutdown(device: Device) {
    return this.run(device, "Shut down", () => api.shutdown(device));
  }

  toast(t: Omit<Toast, "id">) {
    const id = ++this.#toastId;
    this.toasts.push({ ...t, id });
    setTimeout(() => this.dismiss(id), t.kind === "error" ? 10_000 : 6_000);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  addActivity(entry: ActivityEntry) {
    this.activity.push(entry);
    if (this.activity.length > MAX_ACTIVITY) this.activity.splice(0, this.activity.length - MAX_ACTIVITY);
  }

  rememberUrl(url: string) {
    this.settings.recentUrls = [url, ...this.settings.recentUrls.filter((u) => u !== url)].slice(0, 8);
  }
}

export const store = new AppStore();
