import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const styles = readFileSync(`${process.cwd()}/src/styles.css`, "utf8");

function ruleFor(selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = styles.match(new RegExp(`${escaped}\\s*\\{[\\s\\S]*?\\n\\}`));
  return match?.[0] ?? "";
}

describe("quiet visual system contracts", () => {
  it("does not use backdrop blur as the default app shell language", () => {
    expect(ruleFor(".sidebar")).not.toContain("backdrop-filter");
    expect(ruleFor(".assistant-drawer")).not.toContain("backdrop-filter");
    expect(ruleFor(".recommendation-audit-drawer")).not.toContain("backdrop-filter");
  });

  it("keeps cards flatter and avoids decorative hero gradients", () => {
    expect(ruleFor(".hero-panel,\n.panel")).not.toContain("0 24px 80px");
    expect(ruleFor(".hero-panel")).not.toContain("radial-gradient");
  });

  it("uses solid semantic fills instead of gradient status pills", () => {
    expect(ruleFor(".status-pill--running,\n.status-pill--approved")).not.toContain("linear-gradient");
    expect(ruleFor(".segmented-control__button--active,\n.settings-tab--active")).not.toContain("linear-gradient");
  });
});
