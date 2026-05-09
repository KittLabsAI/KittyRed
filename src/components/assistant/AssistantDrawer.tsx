import { CircleStop, RefreshCw, Send, Wrench, X } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties } from "react";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button } from "../../components/ui/button";
import {
  contextShareLabel,
  emptyAssistantContext,
  normalizeAssistantContext,
  type AssistantContextSnapshot,
  type AssistantEvent,
  type AssistantUiMessage,
} from "../../lib/assistantTypes";
import {
  clearAssistantSession,
  listenToAssistantEvents,
  startAssistantRun,
  stopAssistantRun,
} from "../../lib/tauri";
import { useRecommendationStore } from "../../store/recommendationStore";

type AssistantDrawerProps = {
  open: boolean;
  onClose: () => void;
};

const sessionStorageKey = "kittyred:assistant-session";

const markdownComponents: Components = {
  p: ({ node: _node, ...props }) => <p className="leading-7 text-foreground" {...props} />,
  ul: ({ node: _node, ...props }) => <ul className="list-disc space-y-2 pl-5 text-foreground" {...props} />,
  ol: ({ node: _node, ...props }) => <ol className="list-decimal space-y-2 pl-5 text-foreground" {...props} />,
  li: ({ node: _node, ...props }) => <li className="leading-7" {...props} />,
  strong: ({ node: _node, ...props }) => <strong className="font-semibold text-foreground" {...props} />,
  code: ({ node: _node, className, children, ...props }) => {
    const isBlock = Boolean(className);
    if (isBlock) {
      return (
        <code className={className} {...props}>
          {children}
        </code>
      );
    }
    return (
      <code className="rounded-md bg-white/10 px-1.5 py-0.5 text-[0.92em] text-foreground" {...props}>
        {children}
      </code>
    );
  },
  blockquote: ({ node: _node, ...props }) => (
    <blockquote className="border-l border-white/12 pl-4 text-muted-foreground" {...props} />
  ),
};

export function AssistantDrawer({ open, onClose }: AssistantDrawerProps) {
  const assistantDraft = useRecommendationStore((state) => state.assistantDraft);
  const [sessionId] = useState(getOrCreateSessionId);
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<AssistantUiMessage[]>([]);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [context, setContext] = useState<AssistantContextSnapshot>(emptyAssistantContext);
  const messageCounter = useRef(0);
  const restoreFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (open && !input && assistantDraft) {
      setInput(assistantDraft);
    }
  }, [assistantDraft, input, open]);

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    restoreFocusRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;

    return () => {
      restoreFocusRef.current?.focus();
      restoreFocusRef.current = null;
    };
  }, [open]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const handleAssistantEvent = (event: AssistantEvent) => {
      if (event.sessionId !== sessionId) {
        return;
      }

      if (event.context) {
        setContext(normalizeAssistantContext(event.context));
      }

      switch (event.type) {
        case "status":
          if (event.status === "running") {
            setRunning(true);
            setError(null);
          }
          break;
        case "token":
          appendAssistantMessage(event.delta ?? "");
          break;
        case "thinking_status":
        case "thinking_delta":
          upsertThinkingMessage(event);
          break;
        case "tool_start":
          upsertToolMessage(event);
          break;
        case "tool_output":
        case "tool_end":
          updateToolMessage(event);
          break;
        case "done":
          setRunning(false);
          finishAssistantMessage("done", event.reply);
          break;
        case "cancelled":
          setRunning(false);
          finishAssistantMessage("cancelled");
          break;
        case "error":
          setRunning(false);
          setError(event.error ?? "助手运行失败。");
          break;
      }
    };

    void listenToAssistantEvents(handleAssistantEvent).then((cleanup) => {
      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [sessionId]);

  function nextMessageId(prefix: string) {
    messageCounter.current += 1;
    return `${prefix}-${Date.now()}-${messageCounter.current}`;
  }

  function appendAssistantMessage(delta: string) {
    if (!delta) {
      return;
    }
    setMessages((current) => {
      const last = current[current.length - 1];
      if (last?.role === "assistant" && last.status === "running") {
        return [
          ...current.slice(0, -1),
          { ...last, content: `${last.content ?? ""}${delta}` },
        ];
      }
      return [
        ...current,
        {
          id: nextMessageId("assistant"),
          role: "assistant",
          content: delta,
          status: "running",
        },
      ];
    });
  }

  function finishAssistantMessage(status: string, finalContent?: string) {
    setMessages((current) => {
      let finished = false;
      const next = current.map((message) => {
        if (message.role !== "assistant" || message.status !== "running") {
          return message;
        }
        finished = true;
        return {
          ...message,
          content: finalContent ?? message.content,
          status,
        };
      });

      if (finished || !finalContent) {
        return next;
      }

      const last = next[next.length - 1];
      if (
        last?.role === "assistant" &&
        last.status === status &&
        last.content === finalContent
      ) {
        return next;
      }

      return [
        ...next,
        {
          id: nextMessageId("assistant"),
          role: "assistant",
          content: finalContent,
          status,
        },
      ];
    });
  }

  function upsertThinkingMessage(event: AssistantEvent) {
    setMessages((current) => {
      const runningIndex = findLastMessageIndex(
        current,
        (message) => message.role === "thinking" && message.status === "running",
      );
      const lastThinkingIndex = findLastMessageIndex(
        current,
        (message) => message.role === "thinking",
      );
      const index =
        runningIndex >= 0
          ? runningIndex
          : event.type === "thinking_status" && event.status !== "running"
            ? lastThinkingIndex
            : -1;
      const nextContent = event.type === "thinking_delta" ? event.delta ?? "" : "";

      if (index >= 0) {
        const next = [...current];
        next[index] = {
          ...next[index],
          content:
            event.type === "thinking_delta"
              ? `${next[index].content ?? ""}${nextContent}`
              : next[index].content,
          status: event.status ?? next[index].status,
        };
        return next;
      }

      if (event.type === "thinking_status" && event.status !== "running") {
        return current;
      }

      return [
        ...current,
        {
          id: nextMessageId("thinking"),
          role: "thinking",
          content: nextContent,
          status: event.status ?? "running",
          expanded: false,
        },
      ];
    });
  }

  function upsertToolMessage(event: AssistantEvent) {
    const toolCallId = String(event.toolCallId ?? "");
    const args =
      event.arguments && typeof event.arguments === "object" ? event.arguments : {};
    setMessages((current) => {
      const next = finishRunningAssistantMessages(current);
      return [
        ...next,
        {
          id: `tool-${toolCallId || Date.now()}`,
          role: "tool",
          toolCallId,
          name: String(event.name ?? "tool"),
          status: "running",
          summary: String(event.summary ?? event.name ?? "Tool"),
          arguments: args,
          argumentsText: JSON.stringify(args, null, 2),
          output: "",
          resultPreview: "",
          expanded: false,
        },
      ];
    });
  }

  function updateToolMessage(event: AssistantEvent) {
    setMessages((current) => {
      const index = current.findIndex(
        (message) =>
          message.role === "tool" && message.toolCallId === event.toolCallId,
      );
      const output =
        event.type === "tool_end" ? event.resultPreview ?? "" : event.delta ?? "";

      if (index < 0) {
        return [
          ...current,
          {
            id: `tool-${event.toolCallId || Date.now()}`,
            role: "tool",
            toolCallId: event.toolCallId,
            name: event.name ?? "tool",
            status: event.type === "tool_end" ? event.status ?? "done" : "running",
            summary: event.summary ?? event.name ?? "Tool",
            output,
            resultPreview: event.resultPreview ?? "",
            expanded: false,
          },
        ];
      }

      const next = [...current];
      const message = next[index];
      next[index] = {
        ...message,
        status: event.type === "tool_end" ? event.status ?? "done" : message.status,
        output:
          event.type === "tool_end" && !message.output
            ? event.resultPreview ?? ""
            : `${message.output ?? ""}${event.delta ?? ""}`,
        resultPreview: event.resultPreview ?? message.resultPreview,
      };
      return next;
    });
  }

  async function submitMessage() {
    const trimmed = input.trim();
    if (!trimmed || running) {
      return;
    }

    setMessages((current) => [
      ...current,
      { id: nextMessageId("user"), role: "user", content: trimmed },
    ]);
    setError(null);
    setInput("");
    setRunning(true);

    try {
      await startAssistantRun(sessionId, trimmed);
    } catch (submitError) {
      setRunning(false);
      setError(
        submitError instanceof Error
          ? submitError.message
          : "助手运行失败。",
      );
    }
  }

  async function stopCurrentRun() {
    try {
      await stopAssistantRun(sessionId);
    } catch (stopError) {
      setRunning(false);
      setError(
        stopError instanceof Error ? stopError.message : "助手停止失败。",
      );
    }
  }

  async function refreshSession() {
    if (running) {
      await stopAssistantRun(sessionId);
    }
    await clearAssistantSession(sessionId);
    setMessages([]);
    setContext(emptyAssistantContext);
    setInput("");
    setError(null);
    setRunning(false);
  }

  const contextPercent =
    context.maxTokens > 0
      ? Math.min(100, (context.usedTokens / context.maxTokens) * 100)
      : 0;

  if (!open) {
    return null;
  }

  return (
    <section
      aria-modal="true"
      aria-label="AI 助手抽屉"
      className="assistant-drawer border-l border-border bg-[color:var(--panel-strong)] shadow-[-14px_0_42px_rgba(0,0,0,0.26)]"
      role="dialog"
    >
      <header className="assistant-drawer__header border-b border-border">
        <div>
          <span className="assistant-drawer__eyebrow">AI 助手</span>
          <strong className="text-base font-semibold text-foreground">KittyRed 助手</strong>
        </div>
        <div className="assistant-drawer__header-actions">
          <Button
            aria-label="刷新助手"
            className="assistant-round-button"
            onClick={() => void refreshSession()}
            size="icon"
            variant="ghost"
          >
            <RefreshCw size={16} />
          </Button>
          <Button
            aria-label="关闭助手"
            className="assistant-round-button"
            onClick={onClose}
            size="icon"
            variant="ghost"
          >
            <X size={16} />
          </Button>
        </div>
      </header>

      <div className="assistant-drawer__body">
        <div className="assistant-message-list pr-1">
          {messages.length === 0 ? (
            <div className="assistant-empty rounded-2xl border border-white/8 bg-gradient-to-b from-white/4 to-sky-200/8">
              <strong className="text-sm font-semibold text-foreground">可以查询行情、持仓、建议和模拟委托。</strong>
              <span>输入股票代码或投资问题，助手会用中文解释风险和下一步。</span>
            </div>
          ) : (
            messages.map((message) => (
              <AssistantMessageView
                key={message.id}
                message={message}
                onToggle={() =>
                  setMessages((current) =>
                    current.map((item) =>
                      item.id === message.id
                        ? { ...item, expanded: !item.expanded }
                        : item,
                    ),
                  )
                }
              />
            ))
          )}
        </div>
        {error ? (
          <div aria-live="polite" className="assistant-error-banner rounded-2xl border px-4 py-3" role="alert">
            {error}
          </div>
        ) : null}
      </div>

      <footer className="assistant-composer border-t border-border bg-[rgba(5,9,15,0.92)]">
        <label className="sr-only" htmlFor="assistant-message">
          助手消息
        </label>
        <textarea
          aria-label="助手消息"
          className="min-h-[116px] w-full resize-y rounded-2xl border border-border bg-input px-4 py-3 text-sm text-foreground shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          id="assistant-message"
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.preventDefault();
              void submitMessage();
            }
          }}
          placeholder="询问 A 股行情、组合风险、投资建议或模拟委托。"
          rows={4}
          value={input}
        />
        <div className="assistant-composer__actions">
          <div className="assistant-composer__meta">
            {assistantDraft && !input ? (
              <span className="assistant-message__meta">
                已载入最新建议草稿。
              </span>
            ) : (
              <span className="assistant-message__meta">
                按 Cmd/Ctrl+Enter 发送。
              </span>
            )}
          </div>
          <div className="assistant-send-cluster">
            <ContextRing context={context} percent={contextPercent} />
            <Button
              aria-label={running ? "停止" : "发送"}
              className="assistant-round-button assistant-send-button"
              disabled={!running && input.trim().length === 0}
              onClick={() => (running ? void stopCurrentRun() : void submitMessage())}
              size="icon"
            >
              {running ? <CircleStop size={18} /> : <Send size={18} />}
            </Button>
          </div>
        </div>
      </footer>
    </section>
  );
}

function AssistantMessageView({
  message,
  onToggle,
}: {
  message: AssistantUiMessage;
  onToggle: () => void;
}) {
  if (message.role === "user") {
    return (
      <article className="assistant-message assistant-message--user rounded-2xl border bg-white/[0.03] px-4 py-3">
        <div className="assistant-message__meta mb-2">你</div>
        <div className="text-sm leading-6 text-foreground">{message.content}</div>
      </article>
    );
  }

  if (message.role === "assistant") {
    return (
      <article className="assistant-message assistant-message--assistant rounded-2xl border px-4 py-3">
        <div className="assistant-message__meta mb-2">助手</div>
        <div className="assistant-markdown">
          <ReactMarkdown components={markdownComponents} remarkPlugins={[remarkGfm]}>
            {message.content ?? ""}
          </ReactMarkdown>
        </div>
      </article>
    );
  }

  if (message.role === "thinking" || message.role === "tool") {
    const title = message.role === "thinking" ? "思考" : message.name || "工具";
    return (
      <article className={`assistant-fold-card assistant-fold-card--${message.role} rounded-2xl border bg-white/[0.03]`}>
        <Button
          aria-expanded={message.expanded ? "true" : "false"}
          className="assistant-fold-card__button"
          onClick={onToggle}
          variant="ghost"
        >
          <div className="assistant-fold-card__title">
            {message.role === "tool" ? <Wrench size={14} /> : null}
            <strong>{title}</strong>
          </div>
        </Button>
        {message.expanded ? (
          <pre className="assistant-fold-card__content border-t border-white/8 pt-4">
            {message.role === "tool"
              ? [message.argumentsText, message.output || message.resultPreview]
                  .filter(Boolean)
                  .join("\n\n")
              : message.content}
          </pre>
        ) : null}
      </article>
    );
  }

  return <article className="assistant-message assistant-message--error rounded-2xl border px-4 py-3">{message.content}</article>;
}

function ContextRing({
  context,
  percent,
}: {
  context: AssistantContextSnapshot;
  percent: number;
}) {
  return (
    <div
      aria-label="助手上下文用量"
      className="assistant-context-ring"
      role="status"
      style={{ "--context-fill": `${percent}%` } as CSSProperties}
    >
      <span>{Math.round(percent)}%</span>
      <div className="assistant-context-tooltip" role="tooltip">
        <strong>上下文</strong>
        <span>
          {context.usedTokens} / {context.maxTokens || 0} tokens
        </span>
        <span>系统：{contextShareLabel(context, "system")}</span>
        <span>用户：{contextShareLabel(context, "user")}</span>
        <span>助手：{contextShareLabel(context, "assistant")}</span>
        <span>思考：{contextShareLabel(context, "thinking")}</span>
        <span>工具：{contextShareLabel(context, "tool")}</span>
      </div>
    </div>
  );
}

function finishRunningAssistantMessages(messages: AssistantUiMessage[]) {
  return messages.map((message) =>
    message.role === "assistant" && message.status === "running"
      ? { ...message, status: "done" }
      : message,
  );
}

function findLastMessageIndex(
  messages: AssistantUiMessage[],
  predicate: (message: AssistantUiMessage) => boolean,
) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (predicate(messages[index])) {
      return index;
    }
  }
  return -1;
}

function getOrCreateSessionId() {
  if (typeof window === "undefined") {
    return createSessionId();
  }

  const existing = window.sessionStorage.getItem(sessionStorageKey);
  if (existing) {
    return existing;
  }

  const sessionId = createSessionId();
  window.sessionStorage.setItem(sessionStorageKey, sessionId);
  return sessionId;
}

function createSessionId() {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `assistant-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
