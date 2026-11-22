<script lang="ts">
  import type { SystemInfo } from "$lib/api";
  import { bytes, duration } from "$lib/fmt";
  import { toast, fail, ui } from "$lib/ui.svelte";
  import Icon from "$lib/Icon.svelte";

  let { info, onrefresh }: { info: SystemInfo | null; onrefresh: () => void } = $props();

  const memPct = $derived(info ? (info.mem_used / info.mem_total) * 100 : 0);

  function specsText(i: SystemInfo) {
    return [
      `OS: ${i.os} (build ${i.build})`,
      `CPU: ${i.cpu} (${i.cores} cores / ${i.threads} threads)`,
      ...i.gpus.map((g) => `GPU: ${g.name}${g.vram ? ` (${bytes(g.vram)})` : ""}`),
      `RAM: ${bytes(i.mem_total)}`,
      i.board ? `Motherboard: ${i.board}` : "",
      ...i.disks.map((d) => `Disk ${d.mount} ${d.kind} ${bytes(d.total)} (${bytes(d.available)} free)`),
    ]
      .filter(Boolean)
      .join("\n");
  }

  function copy() {
    if (!info) return;
    navigator.clipboard.writeText(specsText(info)).then(() => toast("Specs copied to clipboard"), fail);
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>{info?.hostname ?? "This PC"}</h1>
      <p>{info ? `${info.os} · build ${info.build} · up ${duration(info.uptime)}${info.uptime > 7 * 86400 ? " · consider a restart" : ""}` : "Reading system information…"}</p>
    </div>
    <div class="actions">
      <button onclick={onrefresh}><Icon name="refresh" size={15} /> Refresh</button>
      <button class="primary" onclick={copy} disabled={!info}><Icon name="copy" size={15} /> Copy specs</button>
    </div>
  </div>

  {#if info}
    <div class="grid">
      <div class="card spec">
        <div class="icon"><Icon name="cpu" /></div>
        <div>
          <div class="label">Processor</div>
          <div class="value">{info.cpu}</div>
          <div class="muted">{info.cores} cores · {info.threads} threads · {(info.cpu_mhz / 1000).toFixed(2)} GHz</div>
        </div>
      </div>

      {#each info.gpus as gpu}
        <div class="card spec">
          <div class="icon"><Icon name="monitor" /></div>
          <div>
            <div class="label">Graphics</div>
            <div class="value">{gpu.name}</div>
            <div class="muted">{gpu.vram ? `${bytes(gpu.vram)} dedicated memory` : "Shared memory"}</div>
          </div>
        </div>
      {/each}

      <div class="card spec">
        <div class="icon"><Icon name="ram" /></div>
        <div class="grow">
          <div class="label">Memory</div>
          <div class="value">{bytes(info.mem_total)}</div>
          <div class="bar" class:full={memPct > 85}><div style:width="{memPct}%"></div></div>
          <div class="muted small">{bytes(info.mem_used)} in use ({memPct.toFixed(0)}%){ui.mode === "advanced" ? ` · page file ${bytes(info.swap_total)}` : ""}</div>
        </div>
      </div>

      {#if info.board}
        <div class="card spec">
          <div class="icon"><Icon name="layers" /></div>
          <div>
            <div class="label">Motherboard</div>
            <div class="value">{info.board}</div>
          </div>
        </div>
      {/if}

    </div>

    <h2>Storage</h2>
    <div class="disks">
      {#each info.disks as d}
        {@const used = d.total - d.available}
        {@const pct = d.total ? (used / d.total) * 100 : 0}
        <div class="card disk">
          <div class="disk-head">
            <Icon name="drive" />
            <strong>{d.label || "Local Disk"} ({d.mount.replace(/\\$/, "")})</strong>
            <span class="badge">{d.kind}</span>
            {#if d.removable}<span class="badge">Removable</span>{/if}
            {#if ui.mode === "advanced"}<span class="badge">{d.fs}</span>{/if}
          </div>
          <div class="bar" class:full={pct > 90}><div style:width="{pct}%"></div></div>
          <div class="muted small">{bytes(d.available)} free of {bytes(d.total)}</div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="loading"><div class="spinner"></div></div>
  {/if}
</div>

<style>
  .actions {
    display: flex;
    gap: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  .spec {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .icon {
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .grow {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .value {
    font-size: 16px;
    font-weight: 600;
    margin: 2px 0;
  }
  .small {
    font-size: 12.5px;
  }
  .disks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }
  .disk {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .disk-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .loading {
    display: grid;
    place-items: center;
    padding: 80px;
  }
</style>
