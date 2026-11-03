import type { Attachment } from "svelte/attachments";

const FADE_PX = 28;

/**
 * Fades the edges of a scrolling area where content is hidden past them, instead of cutting
 * it off. Pair with the `scroll-fade` mask in global.css.
 */
export const scrollFade: Attachment<HTMLElement> = (node) => {
  node.classList.add("scroll-fade");
  const update = () => {
    // Grows with the hidden part instead of switching on, which would make the edge jump.
    const hiddenAbove = Math.max(node.scrollTop, 0);
    const hiddenBelow = Math.max(node.scrollHeight - node.clientHeight - node.scrollTop, 0);
    node.style.setProperty("--fade-top", `${Math.min(hiddenAbove, FADE_PX)}px`);
    node.style.setProperty("--fade-bottom", `${Math.min(hiddenBelow, FADE_PX)}px`);
  };

  const resize = new ResizeObserver(update);
  const watchChildren = () => {
    resize.observe(node);
    for (const child of node.children) {
      resize.observe(child);
    }
    update();
  };
  // Tabs and lists swap their children, which the size observer would not see.
  const mutations = new MutationObserver(watchChildren);
  mutations.observe(node, { childList: true });
  node.addEventListener("scroll", update, { passive: true });
  watchChildren();

  return () => {
    resize.disconnect();
    mutations.disconnect();
    node.removeEventListener("scroll", update);
  };
};
