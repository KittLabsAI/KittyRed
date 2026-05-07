import { lazy, Suspense, type ReactNode } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/layout/AppShell";

const DashboardPage = lazy(() =>
  import("./features/dashboard/DashboardPage").then((module) => ({ default: module.DashboardPage })),
);
const MarketsPage = lazy(() =>
  import("./features/markets/MarketsPage").then((module) => ({ default: module.MarketsPage })),
);
const PairDetailPage = lazy(() =>
  import("./features/pair-detail/PairDetailPage").then((module) => ({ default: module.PairDetailPage })),
);
const SignalsPage = lazy(() =>
  import("./features/signals/SignalsPage").then((module) => ({ default: module.SignalsPage })),
);
const PositionsPage = lazy(() =>
  import("./features/positions/PositionsPage").then((module) => ({ default: module.PositionsPage })),
);
const RecommendationsPage = lazy(() =>
  import("./features/recommendations/RecommendationsPage").then((module) => ({ default: module.RecommendationsPage })),
);
const SettingsPage = lazy(() =>
  import("./features/settings/SettingsPage").then((module) => ({ default: module.SettingsPage })),
);

function RouteLoading() {
  return (
    <div aria-live="polite" className="route-loading" role="status">
      页面加载中...
    </div>
  );
}

function lazyPage(page: ReactNode) {
  return (
    <Suspense fallback={<RouteLoading />}>
      {page}
    </Suspense>
  );
}

export function AppRouter() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppShell />} path="/">
          <Route element={lazyPage(<DashboardPage />)} index />
          <Route element={lazyPage(<MarketsPage />)} path="markets" />
          <Route element={lazyPage(<PairDetailPage />)} path="pair-detail" />
          <Route element={lazyPage(<SignalsPage />)} path="signals" />
          <Route element={lazyPage(<PositionsPage />)} path="positions" />
          <Route element={<Navigate replace to="/positions" />} path="orders" />
          <Route element={lazyPage(<RecommendationsPage />)} path="recommendations" />
          <Route element={<Navigate replace to="/recommendations" />} path="recommendations/history" />
          <Route element={lazyPage(<SettingsPage />)} path="settings" />
          <Route element={<Navigate replace to="/" />} path="*" />
        </Route>
      </Routes>
    </HashRouter>
  );
}
