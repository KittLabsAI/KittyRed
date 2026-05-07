import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const styles = readFileSync(`${process.cwd()}/src/styles.css`, "utf8");

function mediaBlock(query) {
  const start = styles.indexOf(`@media (${query})`);
  if (start < 0) return "";
  const next = styles.indexOf("@media", start + 1);
  return styles.slice(start, next < 0 ? undefined : next);
}

describe("responsive adaptation contracts", () => {
  it("turns the mobile sidebar into a compact sticky navigation surface", () => {
    const mobile = mediaBlock("max-width: 640px");

    expect(mobile).toContain(".sidebar");
    expect(mobile).toContain("position: sticky");
    expect(mobile).toContain("max-height");
    expect(mobile).toContain(".sidebar nav ul");
    expect(mobile).toContain("flex-direction: row");
    expect(mobile).toContain("overflow-x: auto");
    expect(mobile).toContain(".sidebar__hint");
    expect(mobile).toContain("display: none");
  });
});
