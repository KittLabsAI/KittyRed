import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const styles = readFileSync(`${process.cwd()}/src/styles.css`, "utf8");

describe("polished loading states", () => {
  it("keeps route loading stable and aligned with the panel system", () => {
    const start = styles.indexOf(".route-loading");
    expect(start).toBeGreaterThanOrEqual(0);

    const block = styles.slice(start, styles.indexOf("}", start));
    expect(block).toContain("min-height: 180px");
    expect(block).toContain("display: grid");
    expect(block).toContain("place-items: center");
    expect(block).toContain("border: 1px solid var(--border)");
    expect(block).toContain("border-radius: 20px");
    expect(block).toContain("background: var(--panel)");
  });
});
