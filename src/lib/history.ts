import type { AfterNavigate } from "$app/navigation";

/** How many steps back the window's history still stays inside the launcher. */
let depth = 0;

/** Call from the root layout's `afterNavigate`. */
export function trackNavigation(navigation: AfterNavigate) {
  const { from, to } = navigation;
  if (navigation.type === "enter" || !from || !to) {
    return;
  }
  if (navigation.type === "popstate") {
    depth = Math.max(0, depth + navigation.delta);
  } else if (from.url.href !== to.url.href) {
    depth += 1;
  }
}

/**
 * Goes back one page the way the browser would, so pressing Back repeatedly walks the whole
 * way home instead of bouncing between the last two pages. Returns false when there is no
 * page to go back to.
 */
export function goBack(): boolean {
  if (depth === 0) {
    return false;
  }
  history.back();
  return true;
}
