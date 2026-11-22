<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { api, type SystemInfo } from "$lib/api";
  import { ui, setMode, fail } from "$lib/ui.svelte";
  import Icon, { type IconName } from "$lib/Icon.svelte";
  import Overview from "$lib/views/Overview.svelte";
  import DiskView from "$lib/views/Disk.svelte";
  import Gaming from "$lib/views/Gaming.svelte";
  import Registry from "$lib/views/Registry.svelte";

  type View = "overview" | "disk" | "gaming" | "registry";
  const nav: { id: View; label: string; icon: IconName; advanced?: boolean }[] = [
    { id: "overview", label: "Overview", icon: "dashboard" },
    { id: "disk", label: "Disk space", icon: "drive" },
    { id: "gaming", label: "Gaming", icon: "gamepad" },
    { id: "registry", label: "Registry", icon: "tree", advanced: true },
  ];

  let view = $state<View>("overview");
  let info = $state<SystemInfo | null>(null);
  let dialog = $state<HTMLDialogElement>();

  // Leaving advanced mode while on an advanced page goes back to the overview.
  $effect(() => {
    if (ui.mode === "normal" && nav.find((n) => n.id === view)?.advanced) view = "overview";
  });

  $effect(() => {
    if (ui.confirm) dialog?.showModal();
    else dialog?.close();
  });

  function answer(yes: boolean) {
    ui.confirm?.resolve(yes);
    ui.confirm = null;
  }

  async function loadInfo() {
    try {
      info = await api.systemInfo();
      document.documentElement.classList.toggle("mica", info.build >= 22000);
    } catch (e) {
      fail(e);
    }
  }

  onMount(() => {
    loadInfo();
    api.isElevated().then((v) => (ui.elevated = v));
  });
</script>

<div class="shell">
  <aside>
    <div class="brand">
      <div class="logo"><Icon name="zap" size={16} /></div>
      <div>
        <div class="brand-name">Windows Manager</div>
        <div class="brand-sub">{info?.os ?? "Windows"}</div>
      </div>
    </div>

    <nav>
      {#each nav.filter((n) => !n.advanced || ui.mode === "advanced") as item (item.id)}
        <button class="nav-item" class:active={view === item.id} onclick={() => (view = item.id)}>
          <Icon name={item.icon} />
          {item.label}
          {#if item.advanced}<span class="badge accent">Pro</span>{/if}
        </button>
      {/each}
    </nav>

    <div class="aside-foot">
      {#if ui.elevated}
        <div class="admin on"><Icon name="shield" size={15} /> Running as administrator</div>
      {:else}
        <button class="admin" onclick={() => api.relaunchAsAdmin().catch(fail)} title="Some features need administrator rights">
          <Icon name="shield" size={15} /> Restart as administrator
        </button>
      {/if}
      <div class="segmented" role="radiogroup" aria-label="Mode">
        <button role="radio" aria-checked={ui.mode === "normal"} class:sel={ui.mode === "normal"} onclick={() => setMode("normal")}>Normal</button>
        <button role="radio" aria-checked={ui.mode === "advanced"} class:sel={ui.mode === "advanced"} onclick={() => setMode("advanced")}>Advanced</button>
      </div>
    </div>
  </aside>

  <main>
    {#if view === "overview"}
      <Overview {info} onrefresh={loadInfo} />
    {:else if view === "disk"}
      <DiskView disks={info?.disks ?? []} />
    {:else if view === "gaming"}
      <Gaming />
    {:else if view === "registry"}
      <Registry />
    {/if}
  </main>
</div>

{#if ui.toast}
  <div class="toast" class:error={ui.toast.kind === "error"} role="status">
    <span>{ui.toast.msg}</span>
    {#if ui.toast.action}
      <button class="ghost" onclick={ui.toast.action.run}>{ui.toast.action.label}</button>
    {/if}
    <button class="ghost" aria-label="Dismiss" onclick={() => (ui.toast = null)}><Icon name="x" size={14} /></button>
  </div>
{/if}

<dialog bind:this={dialog} oncancel={() => answer(false)}>
  {#if ui.confirm}
    <h2>{ui.confirm.title}</h2>
    <p>{ui.confirm.msg}</p>
    <div class="dialog-actions">
      <button onclick={() => answer(false)}>Cancel</button>
      <button class="primary danger-fill" onclick={() => answer(true)}>{ui.confirm.ok}</button>
    </div>
  {/if}
</dialog>

<style>
  .shell {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100vh;
  }
  aside {
    display: flex;
    flex-direction: column;
    padding: 18px 12px 14px;
    gap: 18px;
    border-right: 1px solid var(--border);
    background: rgba(0, 0, 0, 0.18);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px;
  }
  .logo {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: #0b1220;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    box-shadow: 0 4px 16px rgba(96, 165, 250, 0.3);
  }
  .brand-name {
    font-weight: 600;
  }
  .brand-sub {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 160px;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-item {
    position: relative;
    gap: 12px;
    padding: 9px 12px;
    border: none;
    background: transparent;
    color: var(--muted);
    justify-content: flex-start;
  }
  .nav-item .badge {
    margin-left: auto;
  }
  .nav-item:hover:not(:disabled) {
    background: var(--surface);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--surface-2);
    color: var(--text);
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }
  .aside-foot {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .admin {
    font-size: 12.5px;
    justify-content: center;
    color: var(--muted);
    padding: 7px;
  }
  .admin.on {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--ok);
  }
  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 3px;
    gap: 3px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border);
  }
  .segmented button {
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 6px;
    font-size: 13px;
  }
  .segmented button.sel {
    background: var(--surface-3);
    color: var(--text);
  }
  main {
    overflow-y: auto;
    position: relative;
  }
  .toast {
    position: fixed;
    bottom: 20px;
    left: calc(50% + 120px);
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: min(640px, calc(100vw - 300px));
    padding: 8px 8px 8px 16px;
    border-radius: var(--radius);
    background: #1f232b;
    border: 1px solid var(--border);
    border-left: 3px solid var(--ok);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
    animation: rise 0.18s ease-out;
    user-select: text;
    word-break: break-word;
  }
  .toast.error {
    border-left-color: var(--danger);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }
  dialog {
    width: min(440px, 90vw);
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
  dialog p {
    color: var(--muted);
    white-space: pre-line;
    word-break: break-word;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
  .danger-fill {
    background: var(--danger);
  }
  .danger-fill:hover:not(:disabled) {
    background: #fb9393;
  }
</style>
