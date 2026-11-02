const OPEN_MS = 240;

/**
 * Opens `dialog` as a modal and lets it settle in from slightly larger, the way iOS shows an
 * alert.
 *
 * The first frame of a modal is slow on weak PCs because of the blurred backdrop, and a CSS
 * animation would have run its course by then. The dialog stays invisible until that frame
 * is drawn and only then starts to move.
 */
export function openModal(dialog: HTMLDialogElement) {
  dialog.showModal();
  if (document.documentElement.dataset.motion !== "on") {
    return;
  }
  dialog.style.opacity = "0";
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      dialog.style.removeProperty("opacity");
      const timing = { duration: OPEN_MS, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" };
      dialog.animate(
        [
          { opacity: 0, transform: "scale(1.08)" },
          { opacity: 1, transform: "none" },
        ],
        timing,
      );
      dialog.animate([{ opacity: 0 }, { opacity: 1 }], { ...timing, pseudoElement: "::backdrop" });
    }),
  );
}
