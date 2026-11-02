/** Flags that every Java the game runs on, 8 included, understands. */
export const JAVA_FLAGS = {
  g1: ["-XX:+UseG1GC", "-XX:MaxGCPauseMillis=50"],
  dedupe: ["-XX:+UseStringDeduplication"],
  explicitGc: ["-XX:+DisableExplicitGC"],
  preTouch: ["-XX:+AlwaysPreTouch"],
  utf8: ["-Dfile.encoding=UTF-8"],
} as const;

export type JavaFlag = keyof typeof JAVA_FLAGS;

function tokens(args: string): string[] {
  return args.split(/\s+/).filter(Boolean);
}

export function hasFlag(args: string, flag: JavaFlag): boolean {
  const present = tokens(args);
  return JAVA_FLAGS[flag].every((token) => present.includes(token));
}

/** Adds or removes the flag's arguments, one space between each and none at the ends. */
export function toggleFlag(args: string, flag: JavaFlag, on: boolean): string {
  const flagTokens: readonly string[] = JAVA_FLAGS[flag];
  const rest = tokens(args).filter((token) => !flagTokens.includes(token));
  return (on ? [...rest, ...flagTokens] : rest).join(" ");
}
