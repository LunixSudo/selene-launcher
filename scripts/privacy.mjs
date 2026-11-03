// Turns the privacy policy into the plain text the installer shows on its license page.

import { readFileSync, writeFileSync } from "node:fs";

const POLICIES = [
  { from: "PRIVACY.md", to: "src-tauri/nsis/privacy/English.txt", why: "Why", sent: "Sent" },
  {
    from: "PRIVACY.ru.md",
    to: "src-tauri/nsis/privacy/Russian.txt",
    why: "Зачем",
    sent: "Передаётся",
  },
];
const TABLE_RULE = /^\| ?-/;
const BOM = String.fromCodePoint(0xfeff);

function inline(text) {
  return text
    .replace(/\*\*(.+?)\*\*/g, "$1")
    .replace(/`(.+?)`/g, "$1")
    .replace(/\[(.+?)\]\((.+?)\)/g, "$1 ($2)");
}

function plain(markdown, { why, sent }) {
  const source = markdown.trimEnd().split("\n");
  const lines = source.flatMap((line, index) => {
    // The header row only names the columns, and the labels repeat them in every row.
    if (TABLE_RULE.test(line) || TABLE_RULE.test(source[index + 1] ?? "")) {
      return [];
    }
    if (line.startsWith("|")) {
      const [service, reason, data] = line
        .slice(1, -1)
        .split("|")
        .map((cell) => inline(cell.trim()));
      return [`- ${service}. ${why}: ${reason}. ${sent}: ${data}.`];
    }
    return [inline(line.replace(/^#+ /, ""))];
  });
  // NSIS reads the file as UTF-8 only with the byte order mark, and its page wants CRLF.
  return `${BOM}${lines.join("\r\n")}\r\n`;
}

for (const policy of POLICIES) {
  writeFileSync(policy.to, plain(readFileSync(policy.from, "utf8"), policy));
}
