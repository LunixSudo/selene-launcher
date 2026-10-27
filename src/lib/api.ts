import { invoke } from "@tauri-apps/api/core";

export type VersionKind = "release" | "snapshot" | "old_beta" | "old_alpha";

export interface VersionSummary {
  id: string;
  kind: VersionKind;
  releaseTime: string;
}

export interface Instance {
  id: string;
  name: string;
  version: string;
  createdAt: string;
  lastPlayed: string | null;
  maxMemoryMb: number | null;
}

export type ErrorKind =
  | "notSignedIn"
  | "unknownVersion"
  | "unknownInstance"
  | "alreadyRunning"
  | "spawn"
  | "network"
  | "core";

export interface CommandError {
  kind: ErrorKind;
  message: string;
}

export interface LaunchProgress {
  instanceId: string;
  stage: "files" | "java";
  bytesDone: number;
  bytesTotal: number;
}

export interface GameExited {
  instanceId: string;
  code: number | null;
}

export function isCommandError(value: unknown): value is CommandError {
  return typeof value === "object" && value !== null && "kind" in value && "message" in value;
}

export const api = {
  versions: () => invoke<VersionSummary[]>("versions"),
  instances: () => invoke<Instance[]>("instances"),
  createInstance: (name: string, version: string) =>
    invoke<Instance>("create_instance", { name, version }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  launchInstance: (id: string) => invoke<void>("launch_instance", { id }),
};
