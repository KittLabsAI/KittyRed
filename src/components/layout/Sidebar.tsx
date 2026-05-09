import { NavLink } from "react-router-dom";
import { useAppStore } from "../../store/appStore";
import { Button } from "../ui/button";
import { cn } from "../../lib/utils";

type NavigationItem = {
  to: string;
  label: string;
  end?: boolean;
};

export const navigationItems: NavigationItem[] = [
  { to: "/", label: "总览", end: true },
  { to: "/markets", label: "行情" },
  { to: "/pair-detail", label: "个股详情" },
  { to: "/positions", label: "持仓" },
  { to: "/recommendations", label: "AI投资建议", end: true },
  { to: "/backtest", label: "AI回测" },
  { to: "/financial-reports", label: "财报分析" },
  { to: "/signals", label: "策略信号" },
  { to: "/settings", label: "设置" },
];

export function Sidebar() {
  const openAssistant = useAppStore((state) => state.openAssistant);

  return (
    <aside className="sidebar border-border bg-[var(--bg-elevated)] text-foreground">
      <div>
        <div className="sidebar__brand">
          <span className="text-xs uppercase tracking-[0.08em] text-muted-foreground">A股投资助手</span>
          <strong className="text-[1.15rem]">KittyRed</strong>
        </div>
        <nav aria-label="Primary">
          <ul>
            {navigationItems.map((item) => (
              <li key={item.label}>
                <NavLink
                  className={({ isActive }) =>
                    cn(
                      "sidebar__nav-link rounded-lg border border-transparent px-3 py-2 text-sm text-foreground/85 transition-colors duration-150 hover:bg-white/8 hover:text-foreground",
                      isActive && "sidebar__nav-link--active bg-primary text-primary-foreground",
                    )
                  }
                  end={item.end}
                  to={item.to}
                >
                  {item.label}
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>
      </div>
      <div className="sidebar__footer">
        <Button className="sidebar__button w-full" onClick={openAssistant} type="button">
          智能助手
        </Button>
      </div>
    </aside>
  );
}
