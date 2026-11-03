import type { Attachment } from "svelte/attachments";

/** Longer than the entrance animation of the last row, delay included. */
const SETTLE_MS = 800;

/** Marks a list as settled once its rows have come in, so rows added later appear in place. */
export const settle: Attachment<HTMLElement> = (node) => {
  const timer = setTimeout(() => node.classList.add("settled"), SETTLE_MS);
  return () => clearTimeout(timer);
};
