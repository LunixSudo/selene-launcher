import type { Attachment } from "svelte/attachments";

/** How far ahead of the end the next page starts loading. */
const AHEAD_PX = 800;

function scrollParent(node: HTMLElement): HTMLElement | null {
  for (let parent = node.parentElement; parent; parent = parent.parentElement) {
    const { overflowY } = getComputedStyle(parent);
    if (overflowY === "auto" || overflowY === "scroll") {
      return parent;
    }
  }
  return null;
}

/**
 * Calls `reached` once the element scrolls within reach, so a list loads its next page
 * before the player gets to the bottom. Recreate it after each page, as with `{#key}`, to
 * keep loading while the list is still too short to scroll.
 */
export function nearEnd(reached: () => void): Attachment<HTMLElement> {
  return (node) => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          reached();
        }
      },
      { root: scrollParent(node), rootMargin: `0px 0px ${AHEAD_PX}px 0px` },
    );
    observer.observe(node);
    return () => observer.disconnect();
  };
}
