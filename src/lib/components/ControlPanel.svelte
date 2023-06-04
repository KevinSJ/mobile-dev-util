<!-- ControlPanel.svelte -->
<script lang="ts">
  import { Command } from '@tauri-apps/api/shell';

  import { onMount } from 'svelte';
  import { selectedSimulator } from '../store';
  import Modal from './Modal.svelte';

  let operations: {
    name: string;
    command?: any;
    useModal?: boolean;
    handleModalResult?: any;
  }[] = [];

  let modalSubmit:string;

  let currentSimulator: string;

  let isModalOpen: boolean = false;

  function openModal() {
    isModalOpen = true;
  }

  function closeModal() {
    isModalOpen = false;
  }

  selectedSimulator.subscribe((value) => (currentSimulator = value));

  onMount(() => {
    getOperations(); // Fetch the list of simulators on component mount
  });

  async function getOperations(): Promise<void> {
    operations = [
      {
        name: 'Boot simulator',
        command: () => {
          console.log('about to boot ', currentSimulator);
          new Command('emulator', `@${currentSimulator}`).execute();
        },
      },
      {
        name: 'Open deeplink',
        command: () => {
          console.log('about to open modal');
          openModal()
        },
        useModal: true,
        handleModalResult: async (event:CustomEvent<string>) => {
          console.log(`Submitted link: ${event.detail}`);
          modalSubmit = event.detail
          closeModal();
          const cmd =  new Command('adb', `shell am start -d ${modalSubmit}`.split(" "));
          const result = await cmd.execute()
          console.log(result)
          await new Command('adb', `shell am start -d ${modalSubmit}`).execute();
          modalSubmit = ''
        },
      },
      {
        name: 'Shutdown simulator',
        command: () => {
          console.log('about to shutdown', currentSimulator);
          new Command('adb', 'emu kill'.split(' ')).execute();
        },
      },
      {
        name: 'Take screenshot',
        command: () => {
          console.log('about to take screenshot', currentSimulator);
          new Command('emulator', `@${currentSimulator}`).execute();
        },
      },
    ];
  }
</script>

<div class="control-panel">
  {#each operations as operation}
    <button class="control-button" on:click={operation.command}>
      {operation.name}
    </button>
    {#if operation.useModal}
      <Modal
        isOpen={isModalOpen}
        on:closeModal={closeModal}
        on:submit={operation.handleModalResult}
      />
    {/if}
  {/each}
</div>

<style>
  .control-panel {
    display: flex;
    flex-wrap: wrap;
  }

  .control-button {
    flex: 0 0 120px;
    height: 60px;
    margin: 8px;
    background-color: #f0f0f0;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
