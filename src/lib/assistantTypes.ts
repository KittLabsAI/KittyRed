export type AssistantEventType =
  | "status"
  | "token"
  | "thinking_status"
  | "thinking_delta"
  | "tool_start"
  | "tool_output"
  | "tool_end"
  | "done"
  | "cancelled"
  | "error";

export interface AssistantContextBreakdown {
  system: number;
  user: number;
  assistant: number;
  tool: number;
}

export interface AssistantContextSnapshot {
  usedTokens: number;
  maxTokens: number;
  remainingTokens: number;
  thinkingTokens: number;
  breakdown: AssistantContextBreakdown;
}

export interface AssistantEvent {
  sessionId: string;
  type: AssistantEventType;
  delta?: string;
  status?: string;
  reply?: string;
  error?: string;
  context?: unknown;
  toolCallId?: string;
  name?: string;
  summary?: string;
  arguments?: Record<string, unknown>;
  resultPreview?: string;
}

export interface AssistantUiMessage {
  id: string;
  role: "user" | "assistant" | "thinking" | "tool" | "error";
  content?: string;
  status?: string;
  expanded?: boolean;
  toolCallId?: string;
  name?: string;
  summary?: string;
  arguments?: Record<string, unknown>;
  argumentsText?: string;
  output?: string;
  resultPreview?: string;
}

const emptyBreakdown: AssistantContextBreakdown = {
  system: 0,
  user: 0,
  assistant: 0,
  tool: 0,
};

export const emptyAssistantContext: AssistantContextSnapshot = {
  usedTokens: 0,
  maxTokens: 0,
  remainingTokens: 0,
  thinkingTokens: 0,
  breakdown: emptyBreakdown,
};

function numberField(value: unknown): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? Math.round(parsed) : 0;
}

export function normalizeAssistantContext(value: unknown): AssistantContextSnapshot {
  const source = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const rawBreakdown =
    source.breakdown && typeof source.breakdown === "object"
      ? (source.breakdown as Record<string, unknown>)
      : {};
  const breakdown = {
    system: numberField(rawBreakdown.system),
    user: numberField(rawBreakdown.user),
    assistant: numberField(rawBreakdown.assistant),
    tool: numberField(rawBreakdown.tool),
  };
  const breakdownTotal =
    breakdown.system + breakdown.user + breakdown.assistant + breakdown.tool;
  const usedTokens = Math.max(
    numberField(source.usedTokens ?? source.used_tokens),
    breakdownTotal,
  );
  const maxTokens = numberField(source.maxTokens ?? source.max_tokens);
  const remainingTokens =
    maxTokens > 0
      ? Math.max(0, maxTokens - usedTokens)
      : numberField(source.remainingTokens ?? source.remaining_tokens);

  return {
    usedTokens,
    maxTokens,
    remainingTokens,
    thinkingTokens: numberField(source.thinkingTokens ?? source.thinking_tokens),
    breakdown,
  };
}

export function contextShareLabel(
  context: AssistantContextSnapshot,
  key: keyof AssistantContextBreakdown | "thinking",
): string {
  const value = key === "thinking" ? context.thinkingTokens : context.breakdown[key];
  const percent = context.usedTokens > 0 ? (value / context.usedTokens) * 100 : 0;
  return `${percent.toFixed(1)}%`;
}
