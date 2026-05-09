import { NavLink } from "react-router-dom";
import { useAppStore } from "../../store/appStore";

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
    <aside className="sidebar">
      <div>
        <div className="sidebar__brand">
          <span>A股投资助手</span>
          <strong>KittyRed</strong>
        </div>
        <nav aria-label="Primary">
          <ul>
            {navigationItems.map((item) => (
              <li key={item.label}>
                <NavLink
                  className={({ isActive }) => `sidebar__nav-link${isActive ? " sidebar__nav-link--active" : ""}`}
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
        <button className="sidebar__button" onClick={openAssistant} type="button">
          智能助手
        </button>
      </div>
    </aside>
  );
}
