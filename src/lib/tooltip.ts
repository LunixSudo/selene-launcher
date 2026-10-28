import type { Attachment } from "svelte/attachments";

const DELAY_MS = 450;
const GAP = 8;
const MARGIN = 8;

/** A hint shown on hover or keyboard focus, styled like the rest of the app. */
export function tooltip(text: string): Attachment<HTMLElement> {
  return (node) => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let tip: HTMLDivElement | null = null;

    const show = () => {
      if (!text || tip) {
        return;
      }
      tip = document.createElement("div");
      tip.className = "tooltip";
      tip.setAttribute("role", "tooltip");
      tip.textContent = text;
      // Inside a modal dialog only the top layer is visible.
      (node.closest("dialog") ?? document.body).append(tip);

      const anchor = node.getBoundingClientRect();
      const box = tip.getBoundingClientRect();
      const below = anchor.bottom + GAP + box.height <= window.innerHeight - MARGIN;
      const left = Math.min(
        Math.max(anchor.left + anchor.width / 2 - box.width / 2, MARGIN),
        window.innerWidth - box.width - MARGIN,
      );
      tip.style.left = `${left}px`;
      tip.style.top = `${below ? anchor.bottom + GAP : anchor.top - GAP - box.height}px`;
    };

    const hide = () => {
      clearTimeout(timer);
      tip?.remove();
      tip = null;
    };

    const enter = () => {
      clearTimeout(timer);
      timer = setTimeout(show, DELAY_MS);
    };

    const focus = () => {
      if (node.matches(":focus-visible")) {
        show();
      }
    };

    node.addEventListener("pointerenter", enter);
    node.addEventListener("pointerleave", hide);
    node.addEventListener("pointerdown", hide);
    node.addEventListener("focus", focus);
    node.addEventListener("blur", hide);
    return () => {
      hide();
      node.removeEventListener("pointerenter", enter);
      node.removeEventListener("pointerleave", hide);
      node.removeEventListener("pointerdown", hide);
      node.removeEventListener("focus", focus);
      node.removeEventListener("blur", hide);
    };
  };
}
