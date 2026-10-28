<script lang="ts">
  import TextureCrop from "./TextureCrop.svelte";

  let { skin, name, size = 20 }: { skin: string | null; name: string; size?: number } = $props();

  let failed = $state(false);

  $effect(() => {
    void skin;
    failed = false;
  });
</script>

<span class="head" class:letter={!skin || failed} style:--size="{size}px" aria-hidden="true">
  {#if skin && !failed}
    <!-- The face, with the hat layer over it. -->
    <TextureCrop
      src={skin}
      x={8}
      y={8}
      width={8}
      height={8}
      overlay={[40, 8]}
      onerror={() => (failed = true)}
    />
  {:else}
    {name.slice(0, 1)}
  {/if}
</span>

<style>
  .head {
    display: block;
    flex: none;
    width: var(--size);
    height: var(--size);
    overflow: hidden;
    border-radius: calc(var(--size) * 0.25);
  }

  .letter {
    display: grid;
    place-items: center;
    background: var(--accent);
    color: var(--accent-text);
    font-size: calc(var(--size) * 0.55);
    font-weight: 700;
    text-transform: uppercase;
  }
</style>
