/** Flags the player can switch on one by one. All of them work on Java 8 and later unless noted. */
export const JAVA_FLAGS = {
  g1: ["-XX:+UseG1GC", "-XX:MaxGCPauseMillis=50"],
  zgc: ["-XX:+UseZGC"],
  parallelRef: ["-XX:+ParallelRefProcEnabled"],
  dedupe: ["-XX:+UseStringDeduplication"],
  explicitGc: ["-XX:+DisableExplicitGC"],
  preTouch: ["-XX:+AlwaysPreTouch"],
  perfData: ["-XX:+PerfDisableSharedMem"],
  codeCache: ["-XX:ReservedCodeCacheSize=400M", "-XX:+UseCodeCacheFlushing"],
  numa: ["-XX:+UseNUMA"],
  ipv4: ["-Djava.net.preferIPv4Stack=true"],
  utf8: ["-Dfile.encoding=UTF-8"],
  oldForge: [
    "-Dfml.ignoreInvalidMinecraftCertificates=true",
    "-Dfml.ignorePatchDiscrepancies=true",
  ],
} as const;

export type JavaFlag = keyof typeof JAVA_FLAGS;

/** Flags that pick a garbage collector; Java refuses to start with two. */
const COLLECTORS: JavaFlag[] = ["g1", "zgc"];

function tokens(args: string): string[] {
  return args.split(/\s+/).filter(Boolean);
}

export function hasFlag(args: string, flag: JavaFlag): boolean {
  const present = tokens(args);
  return JAVA_FLAGS[flag].every((token) => present.includes(token));
}

/** Adds or removes the flag's arguments, one space between each and none at the ends. */
export function toggleFlag(args: string, flag: JavaFlag, on: boolean): string {
  const removed: string[] = [...JAVA_FLAGS[flag]];
  if (on && COLLECTORS.includes(flag)) {
    for (const other of COLLECTORS) {
      removed.push(...JAVA_FLAGS[other]);
    }
  }
  const rest = tokens(args).filter((token) => !removed.includes(token));
  return (on ? [...rest, ...JAVA_FLAGS[flag]] : rest).join(" ");
}
