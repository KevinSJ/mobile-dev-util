<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { errorMessage, type ActivityEntry, type Platform } from "./lib/api";
  import { store } from "./lib/store.svelte";
  import ActivityLog from "./lib/components/ActivityLog.svelte";
  import DeviceList from "./lib/components/DeviceList.svelte";
  import DevicePanel from "./lib/components/DevicePanel.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import Toasts from "./lib/components/Toasts.svelte";

  const POLL_MS = 3000;

  let settingsOpen = $state(false);
  let activityOpen = $state(false);
  let dragging = $state(false);
  let panel: ReturnType<typeof DevicePanel> | undefined = $state();

  const platforms: { id: Platform; label: string }[] = [
    { id: "android", label: "Android" },
    { id: "ios", label: "iOS" },
  ];

  const mod = $derived(store.env?.os === "macos" ? "⌘" : "Ctrl+");

  const runningCount = (p: Platform) =>
    store.devices[p].filter((d) => store.effectiveState(d) !== "shutdown").length;

  $effect(() => {
    document.documentElement.dataset.platform = store.platform;
  });

  $effect(() => {
    // Persist settings whenever any field changes.
    JSON.stringify(store.settings);
    store.persist();
  });

  function switchPlatform(p: Platform) {
    if (store.platform === p) return;
    store.platform = p;
    void store.refresh(p);
  }

  async function handleDrop(paths: string[]) {
    const device = store.selected;
    if (!device) return;
    if (store.effectiveState(device) !== "booted") {
      store.toast({ kind: "error", title: "Device not running", message: `Boot ${device.name} before installing apps.` });
      return;
    }
    const ext = device.platform === "android" ? ".apk" : ".app";
    const apps = paths.filter((p) => p.toLowerCase().replace(/[\\/]$/, "").endsWith(ext));
    if (!apps.length) {
      store.toast({ kind: "error", title: `Drop ${ext} files to install`, message: "No compatible files in the drop." });
      return;
    }
    for (const path of apps) await panel?.install(device, path);
  }

  onMount(() => {
    const cleanups: (() => void)[] = [];

    store.init().catch((e) => store.toast({ kind: "error", title: "Startup failed", message: errorMessage(e) }));

    // Warm the other platform so its badge count is accurate.
    if (store.platform === "android") setTimeout(() => store.refresh("ios"), 400);
    else setTimeout(() => store.refresh("android"), 400);

    const poll = setInterval(() => {
      if (!document.hidden) void store.refresh();
    }, POLL_MS);
    const clock = setInterval(() => (store.now = Date.now()), 1000);
    const onFocus = () => store.refresh();
    window.addEventListener("focus", onFocus);
    cleanups.push(() => {
      clearInterval(poll);
      clearInterval(clock);
      window.removeEventListener("focus", onFocus);
    });

    listen<ActivityEntry>("activity", (e) => store.addActivity(e.payload)).then((u) => cleanups.push(u));

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") dragging = true;
        else if (p.type === "leave") dragging = false;
        else if (p.type === "drop") {
          dragging = false;
          void handleDrop(p.paths);
        }
      })
      .then((u) => cleanups.push(u));

    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "r") {
        e.preventDefault();
        void store.refresh();
      } else if (mod && e.key === ",") {
        e.preventDefault();
        settingsOpen = true;
      } else if (mod && (e.key === "1" || e.key === "2")) {
        e.preventDefault();
        switchPlatform(e.key === "1" ? "android" : "ios");
      }
    };
    window.addEventListener("keydown", onKey);
    cleanups.push(() => window.removeEventListener("keydown", onKey));

    return () => cleanups.forEach((fn) => fn());
  });
</script>

<div class="shell">
  <header class="topbar">
    <div class="brand">
      <span class="logo"><Icon name="phone" size={15} /></span>
      <h1>Mobile Dev Util</h1>
    </div>

    <div class="segmented" role="tablist" aria-label="Platform">
      {#each platforms as p (p.id)}
        {@const count = runningCount(p.id)}
        <button
          id="tab-{p.id}"
          role="tab"
          class:active={store.platform === p.id}
          aria-selected={store.platform === p.id}
          onclick={() => switchPlatform(p.id)}
        >
          {p.label}
          {#if count}<span class="count">{count}</span>{/if}
        </button>
      {/each}
    </div>

    <div class="tools">
      <button
        id="refresh-button"
        class="btn ghost icon-only"
        title="Refresh ({mod}R)"
        aria-label="Refresh devices"
        onclick={() => store.refresh()}
      >
        <span class="refresh" class:spinning={store.refreshing}><Icon name="refresh" size={15} /></span>
      </button>
      <button
        id="settings-button"
        class="btn ghost icon-only"
        title="Settings ({mod},)"
        aria-label="Settings"
        onclick={() => (settingsOpen = true)}
      >
        <Icon name="settings" size={15} />
      </button>
    </div>
  </header>

  <main class="body">
    <DeviceList onOpenSettings={() => (settingsOpen = true)} />
    {#if store.selected}
      {#key `${store.platform}:${store.selected.id}`}
        <DevicePanel bind:this={panel} device={store.selected} {dragging} />
      {/key}
    {:else}
      <section class="placeholder">
        <span class="ph-icon"><Icon name="phone" size={28} /></span>
        <p class="ph-title">No device selected</p>
        <p class="ph-text">
          Pick an {store.platform === "android" ? "emulator" : "simulator"} on the left to boot it and control it.
        </p>
      </section>
    {/if}
  </main>

  <ActivityLog bind:open={activityOpen} />
</div>

<Toasts />
<SettingsDialog bind:open={settingsOpen} />

<style>
  .shell {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100%;
  }

  .topbar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 12px;
    height: 50px;
    padding: 0 12px 0 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }

  .logo {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: white;
    background: linear-gradient(145deg, color-mix(in srgb, var(--accent) 80%, white), var(--accent));
    box-shadow:
      inset 0 1px 0 hsl(0 0% 100% / 0.25),
      0 1px 3px color-mix(in srgb, var(--accent) 40%, transparent);
    transition: background 0.3s var(--ease);
  }

  h1 {
    font-size: 14px;
    font-weight: 650;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }

  .segmented {
    display: flex;
    padding: 3px;
    gap: 2px;
    border-radius: 9px;
    background: var(--bg-sunken);
    border: 1px solid var(--border);
  }

  .segmented button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 16px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
    cursor: pointer;
    transition:
      background 0.15s var(--ease),
      color 0.15s var(--ease),
      box-shadow 0.15s;
  }

  .segmented button:hover {
    color: var(--text);
  }

  .segmented button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm), 0 0 0 1px var(--border);
  }

  .count {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    display: inline-grid;
    place-items: center;
    font-size: 11px;
    font-weight: 600;
    color: var(--green);
    background: color-mix(in srgb, var(--green) 14%, transparent);
  }

  .tools {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
  }

  .refresh {
    display: grid;
  }

  .refresh.spinning {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .body {
    display: grid;
    grid-template-columns: minmax(240px, 300px) 1fr;
    min-height: 0;
    background: var(--bg);
  }

  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
    color: var(--text-3);
  }

  .ph-icon {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    margin-bottom: 4px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
    color: var(--text-2);
  }

  .ph-title {
    font-weight: 600;
    color: var(--text);
  }

  .ph-text {
    max-width: 300px;
    font-size: 12.5px;
  }
</style>
