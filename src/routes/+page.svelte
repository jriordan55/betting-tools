<script lang="ts">
  // Typed bindings, generated from the Rust command signatures by
  // `cargo test -p bettor-desktop`. Nothing here re-declares a shape the
  // Rust already knows, so drift is a compile error rather than a runtime
  // `undefined`.
  import { commands, type EngineInfo, type Clv } from "$lib/bindings";

  let info = $state<EngineInfo | null>(null);
  let error = $state<string | null>(null);
  let clv = $state<Clv | null>(null);

  $effect(() => {
    commands
      .engineInfo()
      .then((res) => (info = res))
      .catch((e) => (error = String(e)));

    // Phase 3 smoke test: a real math command through the typed boundary.
    // -110 bet into a -130 close, with the other side at +110 so the vig
    // can actually be removed.
    commands.clv(1.909090909090909, 1.7692307692307692, 2.1).then((res) => {
      if (res.status === "ok") clv = res.data;
      else error = res.error.kind;
    });
  });

  const pct = (v: number) => `${(v * 100).toFixed(2)}%`;
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

  {#if clv}
    <h2>CLV on a -110 bet that closed -130</h2>
    <dl>
      <dt>as a ratio</dt>
      <dd>
        {pct(clv.ratio)}
        <span class="note">what the web app showed under three names</span>
      </dd>
      <dt>probability points</dt>
      <dd>
        {pct(clv.probPoints)}
        <span class="note">the measure that compares across prices</span>
      </dd>
      <dt>cents</dt>
      <dd>{clv.cents}</dd>
      <dt>EV vs raw close</dt>
      <dd>
        {pct(clv.evVsRawClose)}
        <span class="note">vig still in — overstated</span>
      </dd>
      <dt>EV vs fair close</dt>
      <dd>{clv.evVsFair === null ? "—" : pct(clv.evVsFair)}</dd>
    </dl>
  {/if}

  <p class="phase">Phase 3 — typed IPC. 32 commands, bindings generated from Rust.</p>
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

  h2 {
    font-size: 1rem;
    font-weight: 500;
    margin: 2rem 0 1rem;
  }

  .note {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .phase {
    margin-top: 2rem;
    color: var(--text-muted);
  }
</style>
