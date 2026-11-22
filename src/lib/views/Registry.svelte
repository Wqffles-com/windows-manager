<script lang="ts">
  import { onMount } from "svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { api, type RegListing, type RegValue } from "$lib/api";
  import { ask, toast, fail } from "$lib/ui.svelte";
  import Icon from "$lib/Icon.svelte";

  const KINDS = ["REG_SZ", "REG_EXPAND_SZ", "REG_MULTI_SZ", "REG_DWORD", "REG_QWORD", "REG_BINARY"];
  const SHORT: Record<string, string> = { HKCR: "HKEY_CLASSES_ROOT", HKCU: "HKEY_CURRENT_USER", HKLM: "HKEY_LOCAL_MACHINE", HKU: "HKEY_USERS", HKCC: "HKEY_CURRENT_CONFIG" };
  const MAX_KEYS = 2000;

  let path = $state("");
  let address = $state("");
  let listing = $state<RegListing>({ subkeys: [], values: [] });
  let filter = $state("");
  let dialog = $state<HTMLDialogElement>();
  let edit = $state<{ mode: "value" | "key"; isNew: boolean; name: string; kind: string; data: string } | null>(null);

  const keys = $derived(listing.subkeys.filter((k) => k.toLowerCase().includes(filter.toLowerCase())));
  const depth = $derived(path ? path.split("\\").length : 0);

  /** Accepts paths pasted from regedit ("Computer\HKLM\...") and short hive names. */
  function normalize(p: string) {
    p = p.trim().replace(/^Computer\\/i, "").replace(/\\+$/, "");
    const [hive, ...rest] = p.split("\\");
    return [SHORT[hive.toUpperCase()] ?? hive.toUpperCase(), ...rest].filter(Boolean).join("\\");
  }

  async function go(p: string) {
    try {
      const target = normalize(p);
      listing = await api.regList(target);
      path = address = target;
      filter = "";
    } catch (e) {
      fail(e);
      address = path;
    }
  }

  const child = (k: string) => (path ? `${path}\\${k}` : k);
  const parent = () => path.split("\\").slice(0, -1).join("\\");

  function shown(v: RegValue) {
    if (v.kind === "REG_DWORD" || v.kind === "REG_QWORD") {
      const n = BigInt(v.data);
      return `0x${n.toString(16).padStart(v.kind === "REG_DWORD" ? 8 : 16, "0")} (${v.data})`;
    }
    return v.kind === "REG_MULTI_SZ" ? v.data.replaceAll("\n", " · ") : v.data;
  }

  function open(e: typeof edit) {
    edit = e;
    dialog?.showModal();
  }

  function saved(undo: string, msg: string) {
    toast(`${msg} An undo file was saved.`, "ok", { label: "Show undo file", run: () => revealItemInDir(undo).catch(fail) });
  }

  async function submit() {
    if (!edit) return;
    try {
      if (edit.mode === "key") {
        await api.regCreateKey(path, edit.name.trim());
        toast(`Created key ${edit.name}`);
      } else {
        saved(await api.regSetValue(path, edit.name, edit.kind, edit.data), `Saved ${edit.name || "(Default)"}.`);
      }
      dialog?.close();
      await go(path);
    } catch (e) {
      fail(e);
    }
  }

  async function deleteValue(v: RegValue) {
    if (!(await ask("Delete value?", `Delete "${v.name || "(Default)"}" from\n${path}?`))) return;
    try {
      saved(await api.regDeleteValue(path, v.name), `Deleted ${v.name || "(Default)"}.`);
      dialog?.close();
      await go(path);
    } catch (e) {
      fail(e);
    }
  }

  async function deleteKey() {
    if (!(await ask("Delete key?", `Delete ${path} and everything under it?\nA full backup (.reg) is exported first.`))) return;
    try {
      saved(await api.regDeleteKey(path), "Key deleted.");
      await go(parent());
    } catch (e) {
      fail(e);
    }
  }

  async function openBackups() {
    try {
      await revealItemInDir(await api.regBackupDir());
    } catch (e) {
      fail(e);
    }
  }

  onMount(() => go(""));
</script>

<div class="page reg">
  <div class="page-head">
    <div>
      <h1>Registry</h1>
      <p>Browse and edit the Windows registry. Every change saves an undo file; double-click it to restore.</p>
    </div>
    <button onclick={openBackups}><Icon name="reveal" size={15} /> Backups</button>
  </div>

  <div class="banner">
    <Icon name="alert" />
    <span>Wrong registry changes can break Windows or apps. Only edit values you understand.</span>
  </div>

  <form class="address" onsubmit={(e) => (e.preventDefault(), go(address))}>
    <button type="button" class="ghost" disabled={!path} onclick={() => go(parent())} aria-label="Parent key"><Icon name="up" size={15} /></button>
    <input class="mono" bind:value={address} placeholder="Computer" spellcheck="false" aria-label="Registry path" />
    <button type="submit">Go</button>
  </form>

  <div class="panes">
    <div class="card pane keys">
      <div class="pane-head">
        <input bind:value={filter} placeholder="Filter {listing.subkeys.length} keys" />
        <button class="ghost" title="New key" disabled={depth < 1} onclick={() => open({ mode: "key", isNew: true, name: "", kind: "", data: "" })}><Icon name="plus" size={15} /></button>
        <button class="ghost danger" title="Delete this key" disabled={depth < 3} onclick={deleteKey}><Icon name="trash" size={15} /></button>
      </div>
      <div class="scroll">
        {#each keys.slice(0, MAX_KEYS) as k (k)}
          <button class="ghost key" onclick={() => go(child(k))}><Icon name="folder" size={15} /><span>{k}</span></button>
        {:else}
          <div class="muted empty">No subkeys</div>
        {/each}
        {#if keys.length > MAX_KEYS}<div class="muted empty">{keys.length - MAX_KEYS} more, use the filter</div>{/if}
      </div>
    </div>

    <div class="card pane values">
      <div class="pane-head">
        <strong>Values</strong>
        <button class="ghost" disabled={depth < 1} onclick={() => open({ mode: "value", isNew: true, name: "", kind: "REG_SZ", data: "" })}><Icon name="plus" size={15} /> New value</button>
      </div>
      <div class="scroll">
        <table>
          <thead><tr><th>Name</th><th>Type</th><th>Data</th></tr></thead>
          <tbody>
            {#each listing.values as v (v.name)}
              <tr onclick={() => open({ mode: "value", isNew: false, name: v.name, kind: KINDS.includes(v.kind) ? v.kind : "REG_BINARY", data: v.data })}>
                <td>{v.name || "(Default)"}</td>
                <td class="muted">{v.kind}</td>
                <td class="mono data">{shown(v)}</td>
              </tr>
            {:else}
              <tr><td colspan="3" class="muted empty">{depth ? "No values" : "Pick a hive to start"}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  </div>
</div>

<dialog bind:this={dialog} onclose={() => (edit = null)}>
  {#if edit}
    <form onsubmit={(e) => (e.preventDefault(), submit())}>
      <h2>{edit.mode === "key" ? "New key" : edit.isNew ? "New value" : "Edit value"}</h2>
      <label>Name <input bind:value={edit.name} disabled={!edit.isNew} placeholder={edit.mode === "value" ? "(Default)" : ""} spellcheck="false" /></label>
      {#if edit.mode === "value"}
        <label>
          Type
          <select bind:value={edit.kind}>
            {#each KINDS as k}<option>{k}</option>{/each}
          </select>
        </label>
        <label>
          Data
          {#if edit.kind === "REG_DWORD" || edit.kind === "REG_QWORD"}
            <input class="mono" bind:value={edit.data} placeholder="Decimal, or hex like 0x1f" spellcheck="false" />
          {:else}
            <textarea class="mono" bind:value={edit.data} rows="5" spellcheck="false" placeholder={edit.kind === "REG_MULTI_SZ" ? "One string per line" : edit.kind === "REG_BINARY" ? "Hex bytes, e.g. 01 a0 ff" : ""}></textarea>
          {/if}
        </label>
      {/if}
      <div class="actions">
        {#if edit.mode === "value" && !edit.isNew}
          {@const name = edit.name}
          <button type="button" class="danger" onclick={() => deleteValue(listing.values.find((v) => v.name === name)!)}><Icon name="trash" size={15} /> Delete</button>
        {/if}
        <span class="spacer"></span>
        <button type="button" onclick={() => dialog?.close()}>Cancel</button>
        <button type="submit" class="primary" disabled={edit.mode === "key" && !edit.name.trim()}>Save</button>
      </div>
    </form>
  {/if}
</dialog>

<style>
  .reg {
    height: 100%;
  }
  .address {
    display: flex;
    gap: 6px;
  }
  .address input {
    flex: 1;
  }
  .panes {
    flex: 1;
    min-height: 320px;
    display: grid;
    grid-template-columns: minmax(220px, 1fr) 2fr;
    gap: 12px;
  }
  .pane {
    display: flex;
    flex-direction: column;
    padding: 0;
    min-height: 0;
    overflow: hidden;
  }
  .pane-head {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }
  .pane-head input {
    flex: 1;
    min-width: 0;
  }
  .values .pane-head {
    justify-content: space-between;
    padding-left: 14px;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding: 4px;
  }
  .key {
    width: 100%;
    justify-content: flex-start;
    padding: 5px 8px;
  }
  .key :global(svg) {
    flex: none;
    color: var(--warn);
  }
  .key span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  th {
    text-align: left;
    font-weight: 600;
    font-size: 12px;
    color: var(--muted);
    padding: 6px 10px;
  }
  th:nth-child(1) {
    width: 32%;
  }
  th:nth-child(2) {
    width: 140px;
  }
  td {
    padding: 6px 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover td {
    background: var(--surface);
  }
  .data {
    user-select: text;
  }
  .empty {
    padding: 16px;
    text-align: center;
  }
  dialog {
    width: min(520px, 90vw);
    padding: 22px;
    color: var(--text);
    background: #1c1f26;
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
  }
  dialog::backdrop {
    background: rgba(0, 0, 0, 0.5);
  }
  dialog form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  dialog label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12.5px;
    color: var(--muted);
  }
  dialog label > :global(*) {
    color: var(--text);
    font-size: 14px;
  }
  textarea {
    resize: vertical;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .spacer {
    flex: 1;
  }
</style>
