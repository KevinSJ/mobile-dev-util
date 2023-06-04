<!-- Main.svelte -->
<script lang="ts">
  import ControlPanel from './lib/components/ControlPanel.svelte';
  import OSswitcher from './lib/components/OsSwitcher.svelte';
  import SimulatorSwitcher from './lib/components/SimulatorSwitcher.svelte';
  import {selectedSimulator} from './lib/store';

  let selectedOS: 'iOS' | 'Android' = 'iOS'; // Default selected OS
  let simulator: string
  selectedSimulator.subscribe((value) => simulator = value)

  function handleOSChange(event: CustomEvent<'iOS' | 'Android'>) {
    selectedOS = event.detail;
  }
</script>

<main>
  <h1>Welcome to the Simulator Control Panel</h1>

  <OSswitcher {selectedOS} on:osChange={handleOSChange} />
  <SimulatorSwitcher {selectedOS} />
  {#if simulator}
    <div class="centered-content">
      <ControlPanel />
    </div>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
  }

  .centered-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
</style>
