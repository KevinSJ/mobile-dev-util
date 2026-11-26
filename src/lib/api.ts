import { invoke } from "@tauri-apps/api/core";

export type Platform = "android" | "ios";
export type DeviceState = "shutdown" | "booting" | "booted" | "shuttingDown";

export interface Device {
  /** AVD name (Android) or UDID (iOS). */
  id: string;
  name: string;
  platform: Platform;
  state: DeviceState;
  runtime: string | null;
  detail: string | null;
  /** adb serial while an Android emulator is running. */
  serial: string | null;
  recording: boolean;
}

export interface Environment {
  os: string;
  androidSdk: string | null;
  adb: string | null;
  emulator: string | null;
  xcrun: string | null;
  sdkOverride: string | null;
  defaultOutputDir: string | null;
  homeDir: string | null;
}

export interface ActivityEntry {
  timestamp: string;
  command: string;
  ok: boolean;
  output: string;
}

export interface DeviceTarget {
  platform: Platform;
  id: string;
  name: string;
  serial: string | null;
}

export const toTarget = (d: Device): DeviceTarget => ({
  platform: d.platform,
  id: d.id,
  name: d.name,
  serial: d.serial,
});

export const deviceKey = (d: Pick<Device, "platform" | "id">) => `${d.platform}:${d.id}`;

const outputDir = (dir: string) => (dir.trim() ? dir : null);

export const api = {
  getEnvironment: () => invoke<Environment>("get_environment"),
  setAndroidSdk: (path: string | null) => invoke<Environment>("set_android_sdk", { path }),
  listDevices: (platform: Platform) => invoke<Device[]>("list_devices", { platform }),
  boot: (d: Device, coldBoot: boolean) =>
    invoke<void>("boot_device", { target: toTarget(d), coldBoot }),
  shutdown: (d: Device) => invoke<void>("shutdown_device", { target: toTarget(d) }),
  wipe: (d: Device) => invoke<void>("wipe_device", { target: toTarget(d) }),
  openUrl: (d: Device, url: string) => invoke<void>("open_url", { target: toTarget(d), url }),
  screenshot: (d: Device, dir: string) =>
    invoke<string>("take_screenshot", { target: toTarget(d), outputDir: outputDir(dir) }),
  install: (d: Device, path: string) => invoke<void>("install_app", { target: toTarget(d), path }),
  startRecording: (d: Device) => invoke<void>("start_recording", { target: toTarget(d) }),
  stopRecording: (d: Device, dir: string) =>
    invoke<string>("stop_recording", { target: toTarget(d), outputDir: outputDir(dir) }),
  reveal: (path: string) => invoke<void>("reveal_path", { path }),
};

export const errorMessage = (e: unknown): string =>
  typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
