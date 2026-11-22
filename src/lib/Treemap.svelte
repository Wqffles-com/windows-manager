<script lang="ts">
  import type { Entry } from "$lib/api";
  import { bytes } from "$lib/fmt";
  import { squarify } from "$lib/treemap";

  let { entries, onopen }: { entries: Entry[]; onopen: (e: Entry) => void } = $props();
  let w = $state(0);
  let h = $state(0);

  // Tiny slivers are unreadable and expensive to draw; the list below shows everything.
  const shown = $derived(entries.filter((e) => e.size > 0).slice(0, 200));
  const rects = $derived(squarify(shown.map((e) => e.size), { x: 0, y: 0, w, h }));
  const color = (e: Entry, i: number) => (e.is_dir ? `hsl(${215 + ((i * 23) % 70)} 70% ${46 - Math.min(i, 12)}%)` : "hsl(220 12% 34%)");
</script>

<div class="map" bind:clientWidth={w} bind:clientHeight={h}>
  {#each rects as r, i (shown[i].name)}
    {@const e = shown[i]}
    <button
      class="tile"
      class:file={!e.is_dir}
      style:left="{r.x}px"
      style:top="{r.y}px"
      style:width="{r.w}px"
      style:height="{r.h}px"
      style:background={color(e, i)}
      title="{e.name} — {bytes(e.size)}"
      onclick={() => onopen(e)}
    >
      {#if r.w > 70 && r.h > 34}
        <span class="name">{e.name}</span>
        <span class="size">{bytes(e.size)}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .map {
    position: relative;
    height: 300px;
    border-radius: var(--radius);
    overflow: hidden;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border);
  }
  .tile {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-start;
    gap: 0;
    padding: 6px 8px;
    border: 1px solid rgba(0, 0, 0, 0.35);
    border-radius: 0;
    overflow: hidden;
    text-align: left;
    transition: filter 0.12s;
  }
  .tile:hover:not(:disabled) {
    filter: brightness(1.25);
    background-color: inherit;
  }
  .tile.file {
    cursor: default;
  }
  .name {
    font-size: 12.5px;
    font-weight: 600;
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #fff;
  }
  .size {
    font-size: 11.5px;
    color: rgba(255, 255, 255, 0.75);
  }
</style>
