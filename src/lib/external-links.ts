import { openUrl } from "@tauri-apps/plugin-opener";
import type { Attachment } from "svelte/attachments";

/** Links inside open in the browser instead of inside the launcher. */
export const externalLinks: Attachment<HTMLElement> = (node) => {
  const open = (event: MouseEvent) => {
    const link = (event.target as HTMLElement).closest("a");
    if (!link || (event.type === "auxclick" && event.button !== 1)) {
      return;
    }
    event.preventDefault();
    if (link.protocol === "https:" || link.protocol === "http:") {
      void openUrl(link.href);
    }
  };
  node.addEventListener("click", open);
  node.addEventListener("auxclick", open);
  return () => {
    node.removeEventListener("click", open);
    node.removeEventListener("auxclick", open);
  };
};
