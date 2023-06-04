<!-- OSswitcher.svelte -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let selectedOS: string;

  const dispatch = createEventDispatcher();

  function handleOSChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    selectedOS = target.value;
    dispatch('osChange', selectedOS); // Emitting 'osChange' event with selected OS
  }
</script>

<div class="os-switcher">
  <label for="os-dropdown">Select OS:</label>
  <select id="os-dropdown" bind:value={selectedOS} on:change={handleOSChange}>
    <option value="iOS">iOS</option>
    <option value="Android">Android</option>
  </select>
</div>

<style>
  /* OSswitcher.css */
  .os-switcher {
    position: absolute;
    top: 20px;
    left: 20px;
    display: flex;
    align-items: center;
  }

  label {
    margin-right: 10px;
  }

  select {
    padding: 5px;
    font-size: 16px;
  }
</style>
