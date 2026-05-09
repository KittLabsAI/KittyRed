import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const routerSource = readFileSync(`${process.cwd()}/src/router.tsx`, "utf8");

describe("router loading performance", () => {
  it("code-splits feature pages instead of importing them into the entry chunk", () => {
    expect(routerSource).toContain("lazy(");
    expect(routerSource).toContain("Suspense");
    expect(routerSource).toContain("import(\"./features/dashboard/DashboardPage\")");
    expect(routerSource).toContain("import(\"./features/financial-reports/FinancialReportsPage\")");
    expect(routerSource).not.toContain("import { DashboardPage }");
    expect(routerSource).not.toContain("features/orders/OrdersPage");
    expect(routerSource).toContain('path="orders"');
    expect(routerSource).toContain('path="financial-reports"');
    expect(routerSource).toContain('to="/positions"');
  });

  it("uses a dedicated route loading surface", () => {
    expect(routerSource).toContain("function RouteLoading");
    expect(routerSource).toContain("className=\"route-loading\"");
    expect(routerSource).toContain("aria-live=\"polite\"");
    expect(routerSource).toContain("页面加载中...");
  });
});
