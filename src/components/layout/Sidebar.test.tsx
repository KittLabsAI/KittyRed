import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("shows the KittyRed A-share brand and Chinese navigation", () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>,
    );

    expect(screen.getByText("KittyRed")).toBeInTheDocument();
    expect(screen.getByText("A股投资助手")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "总览" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "行情" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "个股详情" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "财报分析" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "舆情分析" })).toBeInTheDocument();
    const links = screen.getAllByRole("link").map((link) => link.textContent);
    expect(links.indexOf("财报分析")).toBeGreaterThan(links.indexOf("AI回测"));
    expect(links.indexOf("舆情分析")).toBe(links.indexOf("财报分析") + 1);
    expect(links.indexOf("策略信号")).toBeGreaterThan(links.indexOf("AI投资建议"));
    expect(screen.getByRole("button", { name: "智能助手" })).toBeInTheDocument();
    expect(screen.queryByText("当前仅启用模拟账号。行情和模拟交易数据通过 AKShare 接口接入。")).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "模拟交易" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Spread Monitor" })).not.toBeInTheDocument();
  });

  it("uses the merged AI recommendation navigation item", () => {
    render(
      <MemoryRouter initialEntries={["/recommendations/history"]}>
        <Sidebar />
      </MemoryRouter>,
    );

    expect(screen.getByRole("link", { name: "AI投资建议" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "历史建议" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "投资建议" })).not.toBeInTheDocument();
  });
});
