import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App shell", () => {
  it("renders the primary navigation and assistant entry", () => {
    render(<App />);

    expect(screen.getByRole("navigation", { name: "Primary" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "总览" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "行情" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "订单" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Pair Detail" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "智能助手" })).toBeInTheDocument();
  });
});
