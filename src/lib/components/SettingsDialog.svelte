<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { api, errorMessage } from "../api";
  import { store } from "../store.svelte";
  import Icon from "./Icon.svelte";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let sdk = $state("");
  let outputDir = $state("");

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      sdk = store.settings.androidSdk;
      outputDir = store.settings.outputDir;
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  async function browse(kind: "sdk" | "output") {
    const picked = await openDialog({
      directory: true,
      title: kind === "sdk" ? "Choose the Android SDK folder" : "Choose where to save captures",
    });
    if (typeof picked !== "string") return;
    if (kind === "sdk") sdk = picked;
    else outputDir = picked;
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    try {
      store.settings.androidSdk = sdk.trim();
      store.settings.outputDir = outputDir.trim();
      store.env = await api.setAndroidSdk(store.settings.androidSdk || null);
      open = false;
      store.loaded.android = false;
      await store.refresh("android");
    } catch (err) {
      store.toast({ kind: "error", title: "Could not save settings", message: errorMessage(err) });
    }
  }

  const tools = $derived([
    { label: "Android SDK", value: store.env?.androidSdk },
    { label: "adb", value: store.env?.adb },
    { label: "emulator", value: store.env?.emulator },
    { label: "xcrun (Xcode)", value: store.env?.xcrun, na: store.env?.os !== "macos" },
  ]);
</script>

<dialog bind:this={dialog} onclose={() => (open = false)} onclick={(e) => e.target === dialog && (open = false)}>
  <form class="sheet" onsubmit={save}>
    <header>
      <h2>Settings</h2>
      <button type="button" class="btn ghost icon-only small" aria-label="Close" onclick={() => (open = false)}>
        <Icon name="x" size={14} />
      </button>
    </header>

    <label class="field">
      <span class="label">Android SDK location</span>
      <span class="hint">Leave empty to auto-detect from ANDROID_HOME or the default install location.</span>
      <span class="row">
        <input id="sdk-input" class="input mono" bind:value={sdk} placeholder={store.env?.androidSdk ?? "Auto-detect"} spellcheck="false" />
        <button type="button" class="btn" onclick={() => browse("sdk")}><Icon name="folder" size={14} /> Browse</button>
      </span>
    </label>

    <label class="field">
      <span class="label">Save screenshots & recordings to</span>
      <span class="row">
        <input
          id="output-input"
          class="input mono"
          bind:value={outputDir}
          placeholder={store.env?.defaultOutputDir ?? "Desktop"}
          spellcheck="false"
        />
        <button type="button" class="btn" onclick={() => browse("output")}><Icon name="folder" size={14} /> Browse</button>
      </span>
    </label>

    <div class="field">
      <span class="label">Detected tools</span>
      <ul class="tools">
        {#each tools as t (t.label)}
          <li>
            <span class="status" class:ok={!!t.value} class:na={t.na}>
              <Icon name={t.value ? "check" : t.na ? "x" : "alert"} size={12} />
            </span>
            <span class="tool-label">{t.label}</span>
            <span class="tool-path mono selectable" title={t.value ?? ""}>
              {t.value ?? (t.na ? "macOS only" : "Not found")}
            </span>
          </li>
        {/each}
      </ul>
    </div>

    <footer>
      <button type="button" class="btn" onclick={() => (open = false)}>Cancel</button>
      <button id="settings-save" type="submit" class="btn primary">Save</button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 40px));
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-lg);
  }

  dialog[open] {
    animation: pop 0.2s var(--ease);
  }

  dialog::backdrop {
    background: hsl(220 30% 8% / 0.35);
    backdrop-filter: blur(2px);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }

  .sheet {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 18px 20px 20px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    font-size: 16px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    font-weight: 600;
  }

  .hint {
    margin-top: -3px;
    color: var(--text-3);
    font-size: 12px;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .tools {
    margin: 0;
    padding: 4px 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .tools li {
    display: grid;
    grid-template-columns: 18px 110px 1fr;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    font-size: 12.5px;
  }

  .tools li + li {
    border-top: 1px solid var(--border);
  }

  .status {
    color: var(--amber);
  }

  .status.ok {
    color: var(--green);
  }

  .status.na {
    color: var(--text-3);
  }

  .tool-label {
    color: var(--text-2);
  }

  .tool-path {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-3);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
