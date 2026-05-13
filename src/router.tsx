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
const BacktestPage = lazy(() =>
  import("./features/backtest/BacktestPage").then((module) => ({ default: module.BacktestPage })),
);
const FinancialReportsPage = lazy(() =>
  import("./features/financial-reports/FinancialReportsPage").then((module) => ({ default: module.FinancialReportsPage })),
);
const SentimentAnalysisPage = lazy(() =>
  import("./features/sentiment/SentimentAnalysisPage").then((module) => ({ default: module.SentimentAnalysisPage })),
);
const SettingsPage = lazy(() =>
  import("./features/settings/SettingsPage").then((module) => ({ default: module.SettingsPage })),
);

function RouteLoading() {
  return (
    <div aria-live="polite" className="route-loading grid min-h-[180px] place-items-center rounded-xl border border-border bg-card p-6 text-muted-foreground shadow-[var(--shadow-workbench)]" role="status">
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
          <Route element={lazyPage(<BacktestPage />)} path="backtest" />
          <Route element={lazyPage(<FinancialReportsPage />)} path="financial-reports" />
          <Route element={lazyPage(<SentimentAnalysisPage />)} path="sentiment" />
          <Route element={<Navigate replace to="/recommendations" />} path="recommendations/history" />
          <Route element={lazyPage(<SettingsPage />)} path="settings" />
          <Route element={<Navigate replace to="/" />} path="*" />
        </Route>
      </Routes>
    </HashRouter>
  );
}
