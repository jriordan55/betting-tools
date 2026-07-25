<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type EngineInfo = {
    coreVersion: string;
    shellVersion: string;
    optimized: boolean;
  };

  // Phase 0 gate: this call proves the whole chain is wired —
  // SvelteKit → Tauri IPC → bettor-core. Replaced in Phase 4.
  let info = $state<EngineInfo | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    invoke<EngineInfo>("engine_info")
      .then((res) => (info = res))
      .catch((e) => (error = String(e)));
  });
</script>

<main>
  <h1>bettor<span>-desktop</span></h1>

  {#if error}
    <p class="status err">IPC failed — {error}</p>
  {:else if info}
    <p class="status ok">Rust engine connected</p>
    <dl>
      <dt>math engine</dt>
      <dd>bettor-core {info.coreVersion}</dd>
      <dt>shell</dt>
      <dd>{info.shellVersion}</dd>
      <dt>build</dt>
      <dd>{info.optimized ? "optimized" : "debug (sims will be slow)"}</dd>
    </dl>
  {:else}
    <p class="status">connecting…</p>
  {/if}

  <p class="phase">Phase 0 — scaffold. No math ported yet.</p>
</main>

<style>
  :global(:root) {
    --bg-primary: #0a0a0f;
    --bg-secondary: #12121a;
    --text-primary: #e8e8ed;
    --text-muted: #6b6b7b;
    --accent-green: #2ed573;
    --accent-red: #ff4757;
    --accent-cyan: #00d4aa;
    color-scheme: dark;
  }

  :global(body) {
    margin: 0;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: ui-monospace, "JetBrains Mono", SFMono-Regular, monospace;
    font-size: 14px;
  }

  main {
    max-width: 900px;
    margin: 0 auto;
    padding: 4rem 1.5rem;
  }

  h1 {
    font-size: 1.5rem;
    font-weight: 500;
    letter-spacing: -0.02em;
    margin: 0 0 2rem;
  }

  h1 span {
    color: var(--text-muted);
  }

  .status {
    margin: 0 0 1.5rem;
  }

  .status.ok::before {
    content: "● ";
    color: var(--accent-green);
  }

  .status.err {
    color: var(--accent-red);
  }

  dl {
    display: grid;
    grid-template-columns: 10rem 1fr;
    gap: 0.5rem 1rem;
    margin: 0;
    padding: 1.25rem;
    background: var(--bg-secondary);
    border-radius: 6px;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    color: var(--accent-cyan);
  }

  .phase {
    margin-top: 2rem;
    color: var(--text-muted);
  }
</style>
