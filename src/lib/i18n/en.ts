import type { Messages } from "./index.svelte";

export const en: Messages = {
  window: {
    minimize: "Minimize",
    maximize: "Maximize",
    close: "Close",
    language: "Language",
  },
  instances: {
    title: "Instances",
    create: "New instance",
    empty: "No instances yet",
    emptyHint: "Create your first instance to start playing.",
    play: "Play",
    running: "Running",
    delete: "Delete",
    confirmDelete: "Delete with all worlds?",
    neverPlayed: "Never played",
    lastPlayed: "Played {time}",
  },
  launch: {
    files: "Downloading game {percent}%",
    java: "Downloading Java {percent}%",
    starting: "Starting…",
  },
  create: {
    title: "New instance",
    name: "Name",
    namePlaceholder: "For example, Survival",
    version: "Version",
    snapshots: "Show snapshots",
    loadingVersions: "Loading versions…",
    cancel: "Cancel",
    submit: "Create",
  },
  errors: {
    notSignedIn: "Sign in with a Microsoft account to play.",
    unknownVersion: "This version does not exist.",
    unknownInstance: "Instance not found.",
    alreadyRunning: "This instance is already running.",
    spawn: "Could not start the game.",
    network: "Can't reach the servers. Check your connection.",
    core: "Something went wrong.",
  },
};
