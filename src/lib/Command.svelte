
<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri"

  let command = "";
  let run_result = ""

  async function greet(){
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    run_result = await invoke("run_shell_command_with_result", { command: command })
  }
</script>

<div>
  <form class="row" on:submit|preventDefault={greet}>
    <input id="command-input" placeholder="Enter a shell command" bind:value={command} />
    <button type="submit">Greet</button>
  </form>
  <p>{run_result}</p>
</div>
