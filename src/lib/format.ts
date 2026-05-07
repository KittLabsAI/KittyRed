export function formatCurrency(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: value >= 100 ? 0 : 2,
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

  const year = date.getFullYear();
  const month = padDatePart(date.getMonth() + 1);
  const day = padDatePart(date.getDate());
  const hours = padDatePart(date.getHours());
  const minutes = padDatePart(date.getMinutes());
  const seconds = padDatePart(date.getSeconds());
  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
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

function padDatePart(value: number) {
  return String(value).padStart(2, "0");
}
