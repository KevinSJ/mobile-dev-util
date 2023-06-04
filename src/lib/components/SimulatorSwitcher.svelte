<!-- SimulatorSwitcher.svelte -->
<script lang="ts">
  import { Command } from '@tauri-apps/api/shell';
  import {selectedSimulator} from '../store';

  export let selectedOS: 'iOS' | 'Android';

  $: osType = selectedOS;
  let currentSimulator = '';
  let simulators = [];

  async function fetchSimulators(osType: 'iOS'|'Android') {
    console.log('in SimulatorSwitcher, osType is {}', osType);
    simulators = [];

    switch (osType) {
      case 'iOS':
        console.log('not implemented');
        return { simulators: [], selectedSimulator: '' };
      case 'Android':
        const result = await new Command('emulator', '-list-avds').execute();
        const output = result.stdout;
        simulators = output.split('\n').map((name) => ({ name }));
        currentSimulator = simulators[0].name;

        return { simulators, selectedSimulator: currentSimulator };

      default:
        break;
    }
  }

  $: {
    fetchSimulators(osType).then((result) => {
      simulators = result.simulators;
      currentSimulator = result.selectedSimulator;
      selectedSimulator.set(currentSimulator)
    });
  }

  function handleSimulatorChange(event: Event): void {
    const target = event.target as HTMLSelectElement;
    currentSimulator = target.value;
  }
</script>

<div class="simulator-switcher">
  <label for="simulator-dropdown">Select Simulator:</label>
  <select
    id="simulator-dropdown"
    bind:value={currentSimulator}
    on:change={handleSimulatorChange}
  >
    {#each simulators as simulator}
      <option value={simulator.name}>{simulator.name}</option>
    {/each}
  </select>
</div>

<style>
  .simulator-switcher {
    position: absolute;
    top: 60px;
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
