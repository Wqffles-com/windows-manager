<script lang="ts">
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { api, type Disk, type Entry, type Listing } from "$lib/api";
  import { bytes, num } from "$lib/fmt";
  import { ui, ask, toast, fail } from "$lib/ui.svelte";
  import Icon from "$lib/Icon.svelte";
  import Treemap from "$lib/Treemap.svelte";

  let { disks }: { disks: Disk[] } = $props();

  let phase = $state<"idle" | "scanning" | "done">("idle");
  let scanned = $state(0);
  let root = $state("");
  let custom = $state("");
  let path = $state<string[]>([]);
  let listing = $state<Listing | null>(null);

  async function scan(target: string) {
    target = target.trim();
    if (!target) return;
    phase = "scanning";
    scanned = 0;
    root = target;
    path = [];
    const poll = setInterval(async () => (scanned = await api.scanProgress()), 150);
    try {
      await api.scanStart(target);
      await load();
      phase = "done";
    } catch (e) {
      phase = "idle";
      if (String(e) !== "Scan cancelled") fail(e);
    } finally {
      clearInterval(poll);
    }
  }

  async function load() {
    listing = await api.dirChildren(path);
  }

  function go(to: string[]) {
    path = to;
    load().catch(fail);
  }

  function open(e: Entry) {
    if (e.is_dir) go([...path, e.name]);
  }

  async function reveal(e: Entry) {
    try {
      await revealItemInDir(await api.entryPath([...path, e.name]));
    } catch (err) {
      fail(err);
    }
  }

  async function remove(e: Entry) {
    const what = e.is_dir ? `the folder "${e.name}" and its ${num(e.files)} files` : `"${e.name}"`;
    if (!(await ask("Move to Recycle Bin?", `This moves ${what} (${bytes(e.size)}) to the Recycle Bin.`, "Move to Recycle Bin"))) return;
    try {
      await api.deleteEntry([...path, e.name]);
      await load();
      toast(`Moved ${e.name} to the Recycle Bin`);
    } catch (err) {
      fail(err);
    }
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Disk space</h1>
      <p>Find what is taking up space and clean it up. Deleted items go to the Recycle Bin.</p>
    </div>
    {#if phase === "done"}
      <button onclick={() => scan(root)}><Icon name="refresh" size={15} /> Rescan</button>
    {/if}
  </div>

  {#if phase === "idle"}
    {#if !ui.elevated}
      <div class="banner">
        <Icon name="shield" />
        <span>Without administrator rights, protected system folders are counted as empty.</span>
      </div>
    {/if}
    <div class="drives">
      {#each disks as d}
        {@const pct = d.total ? ((d.total - d.available) / d.total) * 100 : 0}
        <button class="card drive" onclick={() => scan(d.mount)}>
          <div class="drive-head"><Icon name="drive" size={22} /><strong>{d.mount.replace(/\\$/, "")}</strong><span class="muted">{d.label}</span></div>
          <div class="bar" class:full={pct > 90}><div style:width="{pct}%"></div></div>
          <span class="muted">{bytes(d.available)} free of {bytes(d.total)}</span>
          <span class="scan-cta">Scan drive <Icon name="chevron" size={14} /></span>
        </button>
      {/each}
    </div>
    <form class="custom" onsubmit={(e) => (e.preventDefault(), scan(custom))}>
      <input bind:value={custom} placeholder="Or scan a folder, e.g. C:\Users\you\Downloads" />
      <button type="submit" disabled={!custom.trim()}>Scan folder</button>
    </form>
  {:else if phase === "scanning"}
    <div class="card scanning">
      <div class="spinner"></div>
      <div>
        <strong>Scanning {root}</strong>
        <div class="muted">{num(scanned)} items found</div>
      </div>
      <button onclick={() => api.scanCancel()}>Cancel</button>
    </div>
  {:else if listing}
    <div class="crumbs">
      <button class="ghost" disabled={!path.length} onclick={() => go(path.slice(0, -1))} aria-label="Up one folder"><Icon name="up" size={15} /></button>
      <button class="ghost crumb" onclick={() => go([])}>{root.replace(/\\$/, "")}</button>
      {#each path as part, i}
        <Icon name="chevron" size={14} />
        <button class="ghost crumb" onclick={() => go(path.slice(0, i + 1))}>{part}</button>
      {/each}
      <span class="total muted">{bytes(listing.size)} · {num(listing.files)} files</span>
    </div>

    <Treemap entries={listing.entries} onopen={open} />

    <div class="list card">
      {#each listing.entries as e (e.name)}
        {@const pct = listing.size ? (e.size / listing.size) * 100 : 0}
        <div class="row">
          <button class="ghost name" class:dir={e.is_dir} onclick={() => open(e)} disabled={!e.is_dir}>
            <Icon name={e.is_dir ? "folder" : "file"} size={16} />
            <span>{e.name}</span>
          </button>
          <div class="bar"><div style:width="{pct}%"></div></div>
          <span class="size">{bytes(e.size)}</span>
          <span class="pct muted">{pct.toFixed(1)}%</span>
          <span class="files muted">{e.is_dir ? `${num(e.files)} files` : ""}</span>
          <div class="row-actions">
            <button class="ghost" title="Show in Explorer" onclick={() => reveal(e)}><Icon name="reveal" size={15} /></button>
            <button class="ghost danger" title="Move to Recycle Bin" onclick={() => remove(e)}><Icon name="trash" size={15} /></button>
          </div>
        </div>
      {:else}
        <div class="muted empty">This folder is empty.</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .drives {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
  }
  .drive {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    text-align: left;
    padding: 16px;
    background: var(--surface);
  }
  .drive:hover:not(:disabled) {
    border-color: rgba(96, 165, 250, 0.45);
    background: var(--surface-2);
  }
  .drive-head {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
  }
  .scan-cta {
    display: flex;
    align-items: center;
    gap: 2px;
    color: var(--accent);
    font-weight: 600;
    font-size: 13px;
  }
  .custom {
    display: flex;
    gap: 8px;
  }
  .custom input {
    flex: 1;
  }
  .scanning {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .scanning button {
    margin-left: auto;
  }
  .crumbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px;
    color: var(--muted);
  }
  .crumb {
    padding: 4px 8px;
    color: var(--text);
  }
  .total {
    margin-left: auto;
  }
  .list {
    padding: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) 140px 84px 56px 100px auto;
    align-items: center;
    gap: 12px;
    padding: 2px 6px;
    border-radius: 6px;
  }
  .row:hover {
    background: var(--surface);
  }
  .name {
    justify-content: flex-start;
    min-width: 0;
    padding: 6px 4px;
  }
  .name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name:disabled {
    opacity: 1;
  }
  .name.dir {
    color: var(--text);
  }
  .name.dir :global(svg) {
    color: var(--warn);
  }
  .name:not(.dir) :global(svg) {
    color: var(--muted);
  }
  .size {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .pct,
  .files {
    text-align: right;
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
  }
  .row-actions {
    display: flex;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .row:hover .row-actions,
  .row:focus-within .row-actions {
    opacity: 1;
  }
  .row-actions button {
    padding: 5px 7px;
  }
  .empty {
    padding: 20px;
    text-align: center;
  }
</style>
