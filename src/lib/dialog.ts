const OPEN_MS = 220;

/**
 * Opens `dialog` as a modal and lets it fade in, rising a little.
 *
 * The first frame of a modal is slow on weak PCs because of the blurred backdrop, and a CSS
 * animation would have run its course by then. The dialog and its backdrop stay invisible
 * until that frame is drawn and only then start to move. It moves without scaling, which
 * would leave the text a little blurred until the very end and then snap it sharp.
 */
export function openModal(dialog: HTMLDialogElement) {
  if (document.documentElement.dataset.motion !== "on") {
    dialog.showModal();
    return;
  }
  dialog.classList.add("opening");
  dialog.showModal();
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      const timing = { duration: OPEN_MS, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" };
      dialog.animate(
        [
          { opacity: 0, transform: "translateY(10px)" },
          { opacity: 1, transform: "none" },
        ],
        timing,
      );
      dialog.animate([{ opacity: 0 }, { opacity: 1 }], { ...timing, pseudoElement: "::backdrop" });
      dialog.classList.remove("opening");
    }),
  );
}
