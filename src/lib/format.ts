export function formatCurrency(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: value >= 100 ? 0 : 2,
  }).format(value);
}

export function formatStockPrice(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

export function formatPercent(value: number) {
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(2)}%`;
}

export function formatCompact(value: number) {
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

export function formatDateTime(value?: string | number | null) {
  if (value === undefined || value === null || value === "") {
    return "N/A";
  }

  const date = new Date(parseDateTimeInput(value));
  if (Number.isNaN(date.getTime())) {
    return "N/A";
  }

  return formatBeijingDateTime(date);
}

function parseDateTimeInput(value: string | number) {
  if (typeof value === "number") {
    return normalizeEpoch(value);
  }

  const trimmed = value.trim();
  if (trimmed.startsWith("epoch:")) {
    return normalizeEpoch(Number(trimmed.slice("epoch:".length)));
  }

  const numeric = Number(trimmed);
  if (Number.isFinite(numeric) && numeric > 0) {
    return normalizeEpoch(numeric);
  }

  return trimmed;
}

function normalizeEpoch(value: number) {
  return value < 1_000_000_000_000 ? value * 1_000 : value;
}

function formatBeijingDateTime(date: Date) {
  const formatter = new Intl.DateTimeFormat("zh-CN", {
    timeZone: "Asia/Shanghai",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
  const parts = formatter.formatToParts(date);
  const valueOf = (type: Intl.DateTimeFormatPartTypes) => parts.find((part) => part.type === type)?.value ?? "00";
  return `${valueOf("year")}-${valueOf("month")}-${valueOf("day")} ${valueOf("hour")}:${valueOf("minute")}:${valueOf("second")}`;
}
