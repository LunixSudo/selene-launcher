import type { Attachment } from "svelte/attachments";

const FADE_PX = 28;

/**
 * Fades the edges of a scrolling area where content is hidden past them, instead of cutting
 * it off. Pair with the `scroll-fade` mask in global.css.
 */
export const scrollFade: Attachment<HTMLElement> = (node) => {
  node.classList.add("scroll-fade");
  const update = () => {
    const hiddenAbove = node.scrollTop > 0;
    const hiddenBelow = node.scrollTop + node.clientHeight < node.scrollHeight - 1;
    node.style.setProperty("--fade-top", `${hiddenAbove ? FADE_PX : 0}px`);
    node.style.setProperty("--fade-bottom", `${hiddenBelow ? FADE_PX : 0}px`);
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
