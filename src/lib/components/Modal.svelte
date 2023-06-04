<!-- Modal.svelte -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  let link: string = '';
  export let isOpen: boolean = false;

  const dispatch = createEventDispatcher();

  function handleSubmit() {
    dispatch('submit', link);
    link = ''
    dispatch('closeModal');
  }
</script>

<div class="modal" style={isOpen ? 'display: block' : 'display: none'}>
  <div class="modal-content">
    <span class="close" on:click={() => dispatch('closeModal')}>&times;</span>
    <h2>Enter a Link</h2>
    <input type="text" bind:value={link} placeholder="Enter a link" />
    <button on:click={handleSubmit}>Submit</button>
  </div>
</div>

<style>
  .modal {
    display: none;
    position: fixed;
    z-index: 1;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    overflow: auto;
    background-color: rgba(0, 0, 0, 0.5);
  }

  .modal-content {
    background-color: #fefefe;
    margin: 10% auto;
    padding: 20px;
    border: 1px solid #888;
    width: 400px;
  }

  .close {
    color: #aaa;
    float: right;
    font-size: 28px;
    font-weight: bold;
    cursor: pointer;
  }

  .close:hover {
    color: #000;
  }

  input {
    width: 100%;
    padding: 10px;
    margin-bottom: 10px;
  }

  button {
    padding: 10px 20px;
    background-color: #4caf50;
    color: white;
    border: none;
    cursor: pointer;
  }
</style>
