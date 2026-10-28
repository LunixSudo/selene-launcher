import DOMPurify from "dompurify";
import { marked } from "marked";

/** Project descriptions come from mod authors, so everything is sanitized before display. */
export function renderMarkup(body: string, format: "markdown" | "html"): string {
  const html = format === "markdown" ? (marked.parse(body, { async: false }) as string) : body;
  return DOMPurify.sanitize(html, { FORBID_TAGS: ["style", "form", "iframe"] });
}
