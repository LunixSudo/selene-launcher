<script lang="ts">
  /** A moon that waxes from new to full as `percent` goes from 0 to 100. */
  let { percent }: { percent: number } = $props();

  const lit = $derived(Math.min(Math.max(percent, 0), 100) / 100);
  // The lit right half, closed by the terminator: an ellipse that bulges right while the moon
  // is a crescent and left once it is past half.
  const path = $derived(
    `M12 4a8 8 0 0 1 0 16a${(Math.abs(1 - 2 * lit) * 8).toFixed(2)} 8 0 0 ${lit < 0.5 ? 0 : 1} 0-16z`,
  );
</script>

<svg class="moon" viewBox="0 0 24 24" aria-hidden="true">
  <circle cx="12" cy="12" r="8" />
  <path d={path} />
</svg>

<style>
  .moon {
    flex: none;
    width: 18px;
    height: 18px;
  }

  circle {
    fill: currentColor;
    opacity: 0.28;
  }

  path {
    fill: currentColor;
    transition: d 300ms var(--ease);
  }
</style>
