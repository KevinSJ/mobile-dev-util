<script lang="ts">
  import type { DeviceState } from "../api";

  let { state, recording = false }: { state: DeviceState; recording?: boolean } = $props();
</script>

<span class="dot {state}" class:recording title={recording ? "Recording" : state}></span>

<style>
  .dot {
    position: relative;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
    background: var(--border-strong);
    transition: background 0.3s var(--ease);
  }

  .booted {
    background: var(--green);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--green) 18%, transparent);
  }

  .booting,
  .shuttingDown {
    background: var(--amber);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .recording {
    background: var(--red);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--red) 22%, transparent);
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
