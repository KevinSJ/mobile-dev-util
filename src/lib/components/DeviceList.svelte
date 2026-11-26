<script lang="ts">
  import { deviceKey, type Device } from "../api";
  import { store } from "../store.svelte";
  import Icon from "./Icon.svelte";
  import StatusDot from "./StatusDot.svelte";

  let { onOpenSettings }: { onOpenSettings: () => void } = $props();

  let query = $state("");

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const list = store.currentDevices;
    if (!q) return list;
    return list.filter((d) =>
      [d.name, d.id, d.runtime ?? "", d.detail ?? ""].some((s) => s.toLowerCase().includes(q)),
    );
  });

  // iOS: group by runtime. Android: running first, then the rest.
  const groups = $derived.by(() => {
    if (store.platform === "android") {
      const running = filtered.filter((d) => store.effectiveState(d) !== "shutdown");
      const stopped = filtered.filter((d) => store.effectiveState(d) === "shutdown");
      return [
        { label: "Running", devices: running },
        { label: running.length ? "Available" : "", devices: stopped },
      ].filter((g) => g.devices.length);
    }
    const map = new Map<string, Device[]>();
    for (const d of filtered) {
      const key = d.runtime ?? "Other";
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(d);
    }
    return [...map].map(([label, devices]) => ({ label, devices }));
  });

  const error = $derived(store.errors[store.platform]);
  const loaded = $derived(store.loaded[store.platform]);

  function quickToggle(e: MouseEvent, d: Device) {
    e.stopPropagation();
    const devState = store.effectiveState(d);
    if (devState === "shutdown") void store.boot(d);
    else if (devState === "booted") void store.shutdown(d);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const flat = groups.flatMap((g) => g.devices);
    const idx = flat.findIndex((d) => d.id === store.selected?.id);
    const next = flat[Math.max(0, Math.min(flat.length - 1, idx + (e.key === "ArrowDown" ? 1 : -1)))];
    if (next) {
      store.select(next);
      document.getElementById(`device-${CSS.escape(next.id)}`)?.focus();
    }
  }
</script>

<aside class="sidebar">
  <div class="search">
    <span class="search-icon"><Icon name="search" size={14} /></span>
    <input
      id="device-search"
      class="input"
      placeholder={store.platform === "android" ? "Filter emulators" : "Filter simulators"}
      bind:value={query}
      spellcheck="false"
    />
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="list" role="listbox" tabindex="-1" aria-label="Devices" onkeydown={onKeydown}>
    {#if error}
      <div class="empty">
        <Icon name="alert" size={22} />
        <p class="empty-title">{store.platform === "android" ? "Android tools unavailable" : "iOS simulators unavailable"}</p>
        <p class="empty-text selectable">{error}</p>
        {#if store.platform === "android"}
          <button class="btn small" onclick={onOpenSettings}>Set SDK path</button>
        {/if}
      </div>
    {:else if !loaded}
      {#each [0, 1, 2, 3] as i (i)}
        <div class="skeleton" style="animation-delay: {i * 120}ms"></div>
      {/each}
    {:else if store.currentDevices.length === 0}
      <div class="empty">
        <Icon name="phone" size={22} />
        <p class="empty-title">No {store.platform === "android" ? "AVDs" : "simulators"} found</p>
        <p class="empty-text">
          {store.platform === "android"
            ? "Create a virtual device in Android Studio's Device Manager."
            : "Add a simulator in Xcode → Window → Devices and Simulators."}
        </p>
      </div>
    {:else if filtered.length === 0}
      <div class="empty"><p class="empty-text">No devices match “{query}”.</p></div>
    {:else}
      {#each groups as group (group.label)}
        {#if group.label}
          <div class="group-label">{group.label}</div>
        {/if}
        {#each group.devices as d (d.id)}
          {@const devState = store.effectiveState(d)}
          {@const busy = store.busy[deviceKey(d)]}
          <div
            id="device-{d.id}"
            class="row"
            class:selected={store.selected?.id === d.id}
            role="option"
            aria-selected={store.selected?.id === d.id}
            tabindex="0"
            onclick={() => store.select(d)}
            onkeydown={(e) => (e.key === "Enter" || e.key === " ") && store.select(d)}
            ondblclick={() => devState === "shutdown" && store.boot(d)}
          >
            <StatusDot state={devState} recording={d.recording} />
            <div class="meta">
              <span class="name">{d.name}</span>
              <span class="sub">
                {#if store.platform === "android"}
                  {[d.runtime, d.serial].filter(Boolean).join(" · ") || d.id}
                {:else}
                  {devState === "booted" ? "Booted" : devState === "booting" ? "Booting…" : d.id.slice(0, 8)}
                {/if}
              </span>
            </div>
            {#if busy || devState === "booting" || devState === "shuttingDown"}
              <span class="row-action spin"><span class="spinner"></span></span>
            {:else}
              <button
                class="row-action btn small ghost icon-only"
                title={devState === "shutdown" ? "Boot" : "Shut down"}
                aria-label={devState === "shutdown" ? `Boot ${d.name}` : `Shut down ${d.name}`}
                onclick={(e) => quickToggle(e, d)}
              >
                <Icon name={devState === "shutdown" ? "play" : "power"} size={13} />
              </button>
            {/if}
          </div>
        {/each}
      {/each}
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--bg);
  }

  .search {
    position: relative;
    padding: 12px 12px 8px;
  }

  .search .input {
    padding-left: 30px;
    background: var(--surface);
  }

  .search-icon {
    position: absolute;
    left: 22px;
    top: 21px;
    color: var(--text-3);
    pointer-events: none;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 12px;
    outline: none;
  }

  .group-label {
    padding: 12px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 8px 7px 12px;
    margin: 1px 0;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.12s var(--ease);
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .row.selected {
    background: var(--surface);
    box-shadow: var(--shadow-sm), inset 0 0 0 1px var(--border);
  }

  .row.selected::before {
    content: "";
    position: absolute;
    left: 3px;
    top: 9px;
    bottom: 9px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .name {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-action {
    opacity: 0;
    transition: opacity 0.12s;
  }

  .row:hover .row-action,
  .row:focus-visible .row-action,
  .row-action.spin {
    opacity: 1;
  }

  .row-action.spin {
    display: grid;
    place-items: center;
    width: 26px;
    color: var(--amber);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 36px 16px;
    text-align: center;
    color: var(--text-3);
  }

  .empty-title {
    color: var(--text);
    font-weight: 600;
  }

  .empty-text {
    font-size: 12.5px;
    max-width: 240px;
    word-break: break-word;
  }

  .skeleton {
    height: 44px;
    margin: 4px 0;
    border-radius: var(--radius-sm);
    background: linear-gradient(90deg, var(--surface-hover), var(--surface-2), var(--surface-hover));
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }

  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
</style>
