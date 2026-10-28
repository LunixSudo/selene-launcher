// Collects the licenses of everything shipped inside the launcher into
// static/third-party-licenses.txt, which the settings page shows.

import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const TARGET = "x86_64-pc-windows-msvc";
const ROOT_CRATE = "selene-launcher";
const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense)/i;
const OUTPUT = "static/third-party-licenses.txt";

function licenseTexts(dir) {
  return readdirSync(dir)
    .filter((name) => LICENSE_FILE.test(name) && statSync(join(dir, name)).isFile())
    .sort()
    .map((name) => readFileSync(join(dir, name), "utf8").trim());
}

function rustPackages() {
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      ["metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET],
      { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
    ),
  );
  const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const workspace = new Set(metadata.workspace_members);
  const root = metadata.packages.find((pkg) => pkg.name === ROOT_CRATE);

  const seen = new Set();
  const queue = [root.id];
  while (queue.length > 0) {
    const id = queue.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id).deps) {
      if (dep.dep_kinds.some((kind) => kind.kind === null)) queue.push(dep.pkg);
    }
  }

  return [...seen]
    .filter((id) => !workspace.has(id))
    .map((id) => packages.get(id))
    .map((pkg) => ({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license ?? "see the license text",
      source: pkg.repository ?? `https://crates.io/crates/${pkg.name}`,
      texts: licenseTexts(dirname(pkg.manifest_path)),
    }));
}

function jsPackages() {
  const byLicense = JSON.parse(
    execFileSync("pnpm", ["licenses", "list", "--prod", "--json"], {
      encoding: "utf8",
      shell: process.platform === "win32",
    }),
  );
  return Object.values(byLicense)
    .flat()
    .filter((pkg) => !pkg.name.startsWith("@types/"))
    .flatMap((pkg) =>
      pkg.versions.map((version, index) => ({
        name: pkg.name,
        version,
        license: pkg.license,
        source: pkg.homepage ?? `https://www.npmjs.com/package/${pkg.name}`,
        texts: licenseTexts(pkg.paths[index]),
      })),
    );
}

function render(packages) {
  const texts = new Map();
  const numberOf = (text) => {
    const key = text.replace(/\s+/g, " ");
    if (!texts.has(key)) texts.set(key, { number: texts.size + 1, text });
    return texts.get(key).number;
  };
  const lines = packages
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))
    .map((pkg) => {
      const refs = pkg.texts.map((text) => `[${numberOf(text)}]`).join(" ");
      return `${pkg.name} ${pkg.version}, ${pkg.license}, ${pkg.source}${refs ? ` ${refs}` : ""}`;
    });
  const rule = "-".repeat(78);
  return [
    "Selene Launcher includes the following third-party software. The numbers after each",
    "package point to the license texts below.",
    "",
    ...lines,
    ...[...texts.values()].map(({ number, text }) => `\n${rule}\n[${number}]\n\n${text}`),
  ].join("\n");
}

const packages = [...rustPackages(), ...jsPackages()];
writeFileSync(OUTPUT, `${render(packages)}\n`);
console.log(`${packages.length} packages written to ${OUTPUT}`);
