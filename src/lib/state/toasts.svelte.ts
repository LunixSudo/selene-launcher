export interface Toast {
  id: number;
  title: string;
  body: string;
  action?: { label: string; run: () => void };
}

/** At most this many stay on screen; an older one makes room for a new one. */
const MAX_SHOWN = 3;

export const toasts = $state<Toast[]>([]);

let nextId = 0;

/** A short message in the corner that goes away by itself. */
export function toast(message: Omit<Toast, "id">) {
  toasts.push({ ...message, id: nextId++ });
  if (toasts.length > MAX_SHOWN) {
    toasts.shift();
  }
}

export function dismissToast(id: number) {
  const index = toasts.findIndex((entry) => entry.id === id);
  if (index >= 0) {
    toasts.splice(index, 1);
  }
}
