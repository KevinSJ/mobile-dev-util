<script lang="ts">
  import { tick } from "svelte";
  import { store } from "../store.svelte";
  import Icon from "./Icon.svelte";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let scroller: HTMLDivElement | undefined = $state();
  let expanded = $state<Record<number, boolean>>({});

  const failures = $derived(store.activity.filter((e) => !e.ok).length);
  const last = $derived(store.activity.at(-1));

  $effect(() => {
    // Keep the newest entry in view.
    void store.activity.length;
    if (open) tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
  });
</script>

<section class="activity" class:open>
  <button id="activity-toggle" class="bar" onclick={() => (open = !open)} aria-expanded={open}>
    <Icon name="terminal" size={14} />
    <span class="bar-title">Activity</span>
    {#if failures}
      <span class="badge">{failures} failed</span>
    {/if}
    {#if last && !open}
      <span class="last mono" class:fail={!last.ok}>{last.command}</span>
    {/if}
    <span class="spacer"></span>
    {#if open && store.activity.length}
      <span
        class="clear"
        role="button"
        tabindex="0"
        onclick={(e) => {
          e.stopPropagation();
          store.activity = [];
        }}
        onkeydown={(e) => e.key === "Enter" && (store.activity = [])}>Clear</span
      >
    {/if}
    <span class="chev"><Icon name="chevron" size={14} /></span>
  </button>

  {#if open}
    <div class="log selectable" bind:this={scroller}>
      {#if store.activity.length === 0}
        <p class="empty">Commands sent to adb, emulator and simctl will show up here.</p>
      {/if}
      {#each store.activity as entry, i (i)}
        <div class="entry" class:fail={!entry.ok}>
          <button class="line" onclick={() => (expanded[i] = !expanded[i])} disabled={!entry.output}>
            <span class="time mono">{entry.timestamp}</span>
            <span class="status"><Icon name={entry.ok ? "check" : "x"} size={12} /></span>
            <span class="cmd mono">{entry.command}</span>
          </button>
          {#if entry.output && (expanded[i] || !entry.ok)}
            <pre class="out mono">{entry.output}</pre>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .activity {
    border-top: 1px solid var(--border);
    background: var(--bg-sunken);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 14px;
    border: none;
    background: none;
    color: var(--text-2);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }

  .bar:hover {
    color: var(--text);
  }

  .bar-title {
    font-weight: 600;
  }

  .badge {
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--red) 14%, transparent);
    color: var(--red);
    font-size: 11px;
    font-weight: 600;
  }

  .last {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-3);
    font-size: 11.5px;
  }

  .last.fail {
    color: var(--red);
  }

  .spacer {
    flex: 1;
  }

  .clear {
    padding: 2px 6px;
    border-radius: 4px;
  }

  .clear:hover {
    background: var(--surface-hover);
  }

  .chev {
    transform: rotate(180deg);
    transition: transform 0.2s var(--ease);
  }

  .open .chev {
    transform: rotate(0);
  }

  .log {
    height: 190px;
    overflow-y: auto;
    padding: 0 8px 10px;
  }

  .empty {
    padding: 14px 8px;
    color: var(--text-3);
    font-size: 12px;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 3px 6px;
    border: none;
    border-radius: 4px;
    background: none;
    text-align: left;
    font-size: 12px;
    cursor: pointer;
  }

  .line:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .line:disabled {
    cursor: default;
    color: inherit;
  }

  .time {
    color: var(--text-3);
    flex: none;
  }

  .status {
    color: var(--green);
    align-self: center;
  }

  .fail .status,
  .fail .cmd {
    color: var(--red);
  }

  .cmd {
    word-break: break-all;
  }

  .out {
    margin: 2px 0 6px 92px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-2);
    font-size: 11.5px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 160px;
    overflow: auto;
  }
</style>
