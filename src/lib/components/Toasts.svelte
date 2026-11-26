<script lang="ts">
  import { api } from "../api";
  import { store } from "../store.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each store.toasts as t (t.id)}
    <div class="toast {t.kind}" role="status">
      <span class="ic"><Icon name={t.kind === "error" ? "alert" : t.kind === "success" ? "check" : "terminal"} size={15} /></span>
      <div class="body">
        <p class="title">{t.title}</p>
        {#if t.message}<p class="msg selectable">{t.message}</p>{/if}
        {#if t.path}
          <button class="link" onclick={() => api.reveal(t.path!)}>
            <Icon name="folder" size={12} /> Show in folder
          </button>
        {/if}
      </div>
      <button class="close" aria-label="Dismiss" onclick={() => store.dismiss(t.id)}><Icon name="x" size={13} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 48px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(360px, calc(100vw - 32px));
    pointer-events: none;
  }

  .toast {
    display: flex;
    gap: 10px;
    padding: 11px 10px 11px 12px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    pointer-events: auto;
    animation: in 0.25s var(--ease);
  }

  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }

  .ic {
    margin-top: 1px;
    color: var(--accent);
  }

  .success .ic {
    color: var(--green);
  }

  .error .ic {
    color: var(--red);
  }

  .body {
    flex: 1;
    min-width: 0;
  }

  .title {
    font-weight: 600;
  }

  .msg {
    margin-top: 2px;
    color: var(--text-2);
    font-size: 12.5px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 140px;
    overflow: auto;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 6px;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
  }

  .close {
    align-self: flex-start;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--text-3);
    cursor: pointer;
  }

  .close:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
