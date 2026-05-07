import { describe, expect, it } from "vitest";
import { formatDateTime } from "./format";

describe("formatDateTime", () => {
  it("formats ISO, epoch markers, second timestamps, and millisecond timestamps", () => {
    expect(formatDateTime("2026-05-03T19:02:00+08:00")).toBe("2026-05-03 19:02:00");
    expect(formatDateTime("epoch:1777864020")).toBe("2026-05-04 11:07:00");
    expect(formatDateTime(1777864020)).toBe("2026-05-04 11:07:00");
    expect(formatDateTime(1777864020000)).toBe("2026-05-04 11:07:00");
  });

  it("returns N/A for missing or invalid values", () => {
    expect(formatDateTime(undefined)).toBe("N/A");
    expect(formatDateTime("not-a-date")).toBe("N/A");
  });
});
