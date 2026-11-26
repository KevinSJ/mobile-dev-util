<script lang="ts">
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api, deviceKey, type Device } from "../api";
  import { store } from "../store.svelte";
  import Icon from "./Icon.svelte";
  import StatusDot from "./StatusDot.svelte";

  let { device, dragging = false }: { device: Device; dragging?: boolean } = $props();

  let url = $state("");
  let copied = $state(false);

  const key = $derived(deviceKey(device));
  const devState = $derived(store.effectiveState(device));
  const busy = $derived(store.busy[key]);
  const booted = $derived(devState === "booted");
  const isAndroid = $derived(device.platform === "android");
  const outputDir = $derived(store.settings.outputDir || store.env?.defaultOutputDir || "Desktop");
  const prettyOutputDir = $derived(prettyPath(outputDir));
  const elapsed = $derived.by(() => {
    const since = store.recordingSince[key];
    if (!device.recording || !since) return "00:00";
    const s = Math.max(0, Math.floor((store.now - since) / 1000));
    return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  });

  const stateLabel: Record<string, string> = {
    booted: "Running",
    booting: "Booting…",
    shuttingDown: "Shutting down…",
    shutdown: "Stopped",
  };

  function prettyPath(p: string) {
    const home = store.env?.homeDir;
    return home && p.startsWith(home) ? "~" + p.slice(home.length) : p;
  }

  const savedToast = (title: string) => (path: string) => ({
    kind: "success" as const,
    title,
    message: prettyPath(path),
    path,
  });

  async function openLink(e?: SubmitEvent, link = url) {
    e?.preventDefault();
    const value = link.trim();
    if (!value) return;
    const ok = await store.run(device, "Open link", () => api.openUrl(device, value).then(() => true));
    if (ok) {
      store.rememberUrl(value);
      url = "";
    }
  }

  function screenshot() {
    return store.run(device, "Screenshot", () => api.screenshot(device, store.settings.outputDir), savedToast("Screenshot saved"));
  }

  async function toggleRecording() {
    if (device.recording) {
      await store.run(device, "Save recording", () => api.stopRecording(device, store.settings.outputDir), savedToast("Recording saved"));
      delete store.recordingSince[key];
    } else {
      const ok = await store.run(device, "Start recording", () => api.startRecording(device).then(() => true));
      if (ok) store.recordingSince[key] = Date.now();
    }
  }

  async function pickAndInstall() {
    const path = await open({
      multiple: false,
      directory: false,
      title: isAndroid ? "Choose an APK to install" : "Choose a .app bundle to install",
      filters: isAndroid
        ? [{ name: "Android package", extensions: ["apk"] }]
        : [{ name: "iOS simulator app", extensions: ["app"] }],
    });
    if (typeof path === "string") await install(device, path);
  }

  async function wipe() {
    const confirmed = await ask(
      isAndroid
        ? `All data on “${device.name}” will be erased and the emulator will boot fresh.`
        : `All content and settings on “${device.name}” will be erased.`,
      { title: isAndroid ? "Wipe data?" : "Erase simulator?", kind: "warning", okLabel: isAndroid ? "Wipe & Boot" : "Erase" },
    );
    if (!confirmed) return;
    if (isAndroid) store.bootRequested[key] = Date.now();
    await store.run(device, isAndroid ? "Wipe data" : "Erase", () => api.wipe(device), () => ({
      kind: "success",
      title: isAndroid ? "Booting with wiped data" : "Simulator erased",
    }));
  }

  async function copyId() {
    await navigator.clipboard.writeText(device.serial ?? device.id);
    copied = true;
    setTimeout(() => (copied = false), 1200);
  }

  export function install(target: Device, path: string) {
    const file = path.split(/[\\/]/).pop() ?? path;
    return store.run(target, "Install", () => api.install(target, path), () => ({
      kind: "success",
      title: "App installed",
      message: `${file} → ${target.name}`,
    }));
  }
</script>

<section class="panel">
  <header class="head">
    <div class="title-block">
      <div class="title-row">
        <h2 class="selectable">{device.name}</h2>
        <span class="state-chip {devState}" class:rec={device.recording}>
          <StatusDot state={devState} recording={device.recording} />
          {device.recording ? `Recording ${elapsed}` : stateLabel[devState]}
        </span>
      </div>
      <div class="facts">
        {#if device.runtime}<span>{device.runtime}</span>{/if}
        {#if device.detail}<span>{device.detail}</span>{/if}
        <button class="id mono" title="Copy identifier" onclick={copyId}>
          {device.serial ?? device.id}
          <Icon name={copied ? "check" : "copy"} size={12} />
        </button>
      </div>
    </div>

    <div class="lifecycle">
      {#if devState === "shutdown"}
        {#if isAndroid}
          <label class="toggle" title="Ignore the saved snapshot and perform a full boot">
            <input type="checkbox" bind:checked={store.settings.coldBoot} />
            <Icon name="snowflake" size={13} /> Cold boot
          </label>
        {/if}
        <button id="boot-button" class="btn primary" disabled={!!busy} onclick={() => store.boot(device)}>
          {#if busy === "Boot"}<span class="spinner"></span>{:else}<Icon name="play" size={14} />{/if}
          Boot
        </button>
      {:else}
        <button
          id="shutdown-button"
          class="btn"
          disabled={!!busy || devState !== "booted"}
          onclick={() => store.shutdown(device)}
        >
          {#if busy === "Shut down" || devState === "shuttingDown"}<span class="spinner"></span>{:else}<Icon name="power" size={14} />{/if}
          Shut down
        </button>
      {/if}
      <button
        id="wipe-button"
        class="btn danger icon-only"
        title={devState !== "shutdown"
          ? "Shut the device down to " + (isAndroid ? "wipe its data" : "erase it")
          : isAndroid ? "Wipe data & boot" : "Erase content & settings"}
        aria-label={isAndroid ? "Wipe data" : "Erase"}
        disabled={!!busy || devState !== "shutdown"}
        onclick={wipe}
      >
        <Icon name="eraser" size={15} />
      </button>
    </div>
  </header>

  {#if !booted}
    <div class="hint">
      {#if devState === "booting"}
        <span class="spinner"></span> Waiting for {device.name} to finish booting…
      {:else}
        <Icon name="alert" size={14} /> Boot the device to open links, capture the screen or install apps.
      {/if}
    </div>
  {/if}

  <div class="cards" class:disabled={!booted}>
    <article class="card wide">
      <div class="card-head">
        <Icon name="link" />
        <h3>Open link</h3>
        <span class="card-sub">URL or deep link, e.g. <span class="mono">myapp://settings</span></span>
      </div>
      <form class="url-row" onsubmit={openLink}>
        <input
          id="url-input"
          class="input mono"
          placeholder="https://example.com or myapp://path"
          bind:value={url}
          disabled={!booted}
          spellcheck="false"
          autocomplete="off"
        />
        <button id="open-url-button" class="btn primary" type="submit" disabled={!booted || !url.trim() || !!busy}>
          {#if busy === "Open link"}<span class="spinner"></span>{/if}
          Open
        </button>
      </form>
      {#if store.settings.recentUrls.length}
        <div class="recents">
          {#each store.settings.recentUrls as recent (recent)}
            <button class="chip mono" title={recent} disabled={!booted || !!busy} onclick={() => openLink(undefined, recent)}>
              {recent}
            </button>
          {/each}
        </div>
      {/if}
    </article>

    <article class="card">
      <div class="card-head">
        <Icon name="camera" />
        <h3>Capture</h3>
      </div>
      <div class="capture-row">
        <button id="screenshot-button" class="btn" disabled={!booted || !!busy} onclick={screenshot}>
          {#if busy === "Screenshot"}<span class="spinner"></span>{:else}<Icon name="camera" size={14} />{/if}
          Screenshot
        </button>
        <button
          id="record-button"
          class="btn record"
          class:active={device.recording}
          disabled={(!booted && !device.recording) || (!!busy && !device.recording)}
          onclick={toggleRecording}
        >
          {#if busy === "Save recording" || busy === "Start recording"}
            <span class="spinner"></span>
          {:else if device.recording}
            <Icon name="stop" size={13} />
          {:else}
            <span class="rec-dot"></span>
          {/if}
          {device.recording ? `Stop · ${elapsed}` : "Record"}
        </button>
      </div>
      <p class="card-foot">
        <Icon name="folder" size={12} />
        <span class="selectable" title={outputDir}>Saves to {prettyOutputDir}</span>
      </p>
      {#if isAndroid}
        <p class="card-foot muted">Android screen recordings stop automatically after 3 minutes.</p>
      {/if}
    </article>

    <article class="card">
      <div class="card-head">
        <Icon name="upload" />
        <h3>Install app</h3>
      </div>
      <button id="install-button" class="dropzone" class:dragging disabled={!booted || !!busy} onclick={pickAndInstall}>
        {#if busy === "Install"}
          <span class="spinner"></span>
          <span>Installing…</span>
        {:else}
          <Icon name="upload" size={20} />
          <span>Drop {isAndroid ? "an .apk" : "a .app bundle"} here</span>
          <span class="muted">or click to browse</span>
        {/if}
      </button>
    </article>
  </div>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 26px 26px;
    min-height: 0;
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .title-block {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  h2 {
    font-size: 21px;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.2;
  }

  .state-chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 22px;
    padding: 0 9px 0 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
    background: var(--surface-hover);
  }

  .state-chip.booted {
    color: var(--green);
    background: color-mix(in srgb, var(--green) 12%, transparent);
  }

  .state-chip.booting,
  .state-chip.shuttingDown {
    color: var(--amber);
    background: color-mix(in srgb, var(--amber) 13%, transparent);
  }

  .state-chip.rec {
    color: var(--red);
    background: color-mix(in srgb, var(--red) 12%, transparent);
    font-variant-numeric: tabular-nums;
  }

  .facts {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 14px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .id {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    margin-left: -6px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--text-3);
    cursor: pointer;
  }

  .id:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .lifecycle {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 6px;
    color: var(--text-2);
    font-size: 12.5px;
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--accent);
    margin: 0;
  }

  .hint {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-sunken);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
    transition: opacity 0.2s var(--ease);
  }

  .cards.disabled {
    opacity: 0.6;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
  }

  .card.wide {
    grid-column: 1 / -1;
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }

  .card-head h3 {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text);
  }

  .card-sub {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .url-row {
    display: flex;
    gap: 8px;
  }

  .recents {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    max-width: 260px;
    height: 24px;
    padding: 0 9px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    transition:
      background 0.12s,
      color 0.12s,
      border-color 0.12s;
  }

  .chip:hover:not(:disabled) {
    color: var(--text);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: var(--accent-soft);
  }

  .chip:disabled {
    cursor: not-allowed;
  }

  .capture-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .capture-row .btn {
    flex: 1;
  }

  .rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--red);
  }

  .record.active {
    color: var(--red);
    border-color: color-mix(in srgb, var(--red) 40%, var(--border));
    background: var(--red-soft);
    font-variant-numeric: tabular-nums;
  }

  .card-foot {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: auto;
    font-size: 12px;
    color: var(--text-3);
    min-width: 0;
  }

  .card-foot span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-foot.muted {
    margin-top: -6px;
  }

  .muted {
    color: var(--text-3);
    font-size: 12px;
  }

  .dropzone {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 92px;
    border-radius: var(--radius-sm);
    border: 1.5px dashed var(--border-strong);
    background: var(--surface-2);
    color: var(--text-2);
    cursor: pointer;
    transition:
      background 0.15s var(--ease),
      border-color 0.15s var(--ease),
      color 0.15s;
  }

  .dropzone:hover:not(:disabled),
  .dropzone.dragging:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text);
  }

  .dropzone:disabled {
    cursor: not-allowed;
  }

  @media (max-width: 900px) {
    .cards {
      grid-template-columns: 1fr;
    }
    .card-sub {
      display: none;
    }
  }
</style>
