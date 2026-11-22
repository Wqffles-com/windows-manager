<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Tweak } from "$lib/api";
  import { ui, toast, fail } from "$lib/ui.svelte";
  import Icon from "$lib/Icon.svelte";

  let tweaks = $state<Tweak[]>([]);
  let busy = $state<string | null>(null);

  const categories = $derived([...new Set(tweaks.map((t) => t.category))]);
  const needsAdmin = $derived(!ui.elevated && tweaks.some((t) => t.admin));

  async function load() {
    tweaks = await api.tweaks().catch((e) => (fail(e), []));
  }

  async function toggle(t: Tweak) {
    busy = t.id;
    try {
      await api.tweakSet(t.id, !t.enabled);
      await load();
      toast(t.restart ? `${t.title} ${t.enabled ? "turned off" : "applied"}. Restart your PC for it to take effect.` : `${t.title} ${t.enabled ? "turned off" : "applied"}.`);
    } catch (e) {
      fail(e);
    } finally {
      busy = null;
    }
  }

  onMount(load);
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Gaming</h1>
      <p>Tune Windows for lower latency and higher FPS. Every switch can be turned back off to restore the Windows default.</p>
    </div>
  </div>

  {#if needsAdmin}
    <div class="banner">
      <Icon name="shield" />
      <span>Some tweaks change system-wide settings and need administrator rights.</span>
      <button onclick={() => api.relaunchAsAdmin().catch(fail)}>Restart as administrator</button>
    </div>
  {/if}

  {#each categories as cat}
    <section>
      <h2>{cat}</h2>
      <div class="list">
        {#each tweaks.filter((t) => t.category === cat) as t (t.id)}
          {@const locked = t.admin && !ui.elevated}
          <label class="card tweak" class:locked>
            <div class="text">
              <div class="title">
                {t.title}
                {#if t.admin}<span class="badge">Admin</span>{/if}
                {#if t.restart}<span class="badge warn">Restart</span>{/if}
              </div>
              <div class="muted">{t.desc}</div>
            </div>
            {#if busy === t.id}
              <div class="spinner"></div>
            {:else}
              <input type="checkbox" role="switch" class="switch" checked={t.enabled} disabled={locked || busy !== null} onchange={() => toggle(t)} />
            {/if}
          </label>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .tweak {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 14px 18px;
    cursor: pointer;
    transition: background 0.12s;
  }
  .tweak:hover {
    background: var(--surface-2);
  }
  .tweak.locked {
    cursor: default;
    opacity: 0.6;
  }
  .text {
    flex: 1;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    margin-bottom: 2px;
  }
  .switch {
    appearance: none;
    flex: none;
    width: 40px;
    height: 20px;
    margin: 0;
    padding: 0;
    border-radius: 99px;
    border: 1px solid rgba(255, 255, 255, 0.45);
    background: transparent;
    position: relative;
    cursor: inherit;
    transition: background 0.15s, border-color 0.15s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.8);
    transition: transform 0.15s;
  }
  .switch:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  .switch:checked::after {
    transform: translateX(20px);
    background: #08111f;
  }
</style>
