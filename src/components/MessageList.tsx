import { Check, Copy, FileText, Paperclip, RotateCcw } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { api } from "@/lib/api";
import { formatBytes, initials, relativeTime } from "@/lib/format";
import { useChatStore } from "@/stores/chat";
import { useProvidersStore } from "@/stores/providers";
import type { Attachment, MessageView } from "@/types";

import { Markdown } from "./Markdown";

function AttachmentPreview({ attachment }: { attachment: Attachment }) {
  const [preview, setPreview] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let live = true;
    if (attachment.kind !== "image") return;
    api
      .attachmentPreview(attachment.id)
      .then((value) => {
        if (live) setPreview(value);
      })
      .catch(() => {
        if (live) setFailed(true);
      });
    return () => {
      live = false;
    };
  }, [attachment.id, attachment.kind]);

  if (attachment.kind === "image" && preview && !failed) {
    return (
      <img
        className="message-image"
        src={preview}
        alt={attachment.name}
        loading="lazy"
      />
    );
  }
  return (
    <span className="attach-chip" title={attachment.path}>
      {attachment.kind === "text" ? <FileText size={11} /> : <Paperclip size={11} />}
      <span className="name">{attachment.name}</span>
      <span className="faint">{formatBytes(attachment.size)}</span>
    </span>
  );
}

function MessageRow({ view }: { view: MessageView }) {
  const message = view.message;
  const assistant = message.role === "assistant";
  const streaming = useChatStore((state) => state.streaming);
  const refresh = useChatStore((state) => state.refreshMessages);
  const continueResponse = useChatStore((state) => state.continueResponse);
  const accounts = useProvidersStore((state) => state.accounts);
  const [copied, setCopied] = useState(false);
  const accountLabel = accounts.find(
    (account) => account.id === message.accountId,
  )?.label;
  const isActiveStream =
    assistant && streaming.active && streaming.assistantId === message.id;
  const content = message.content;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard access can be denied; no user action is required.
    }
  };

  return (
    <div className={`message ${assistant ? "assistant" : "user"}`}>
      <div className="avatar">{assistant ? "CM" : initials("You")}</div>
      <div className="message-main">
        <div className="message-meta">
          <span className="message-author">{assistant ? "Claude Multi" : "You"}</span>
          {assistant && message.model ? (
            <span className="faint small-text">
              {message.model}
              {accountLabel ? ` · ${accountLabel}` : ""}
            </span>
          ) : null}
          <span className="faint small-text">{relativeTime(message.createdAt)}</span>
          {message.status === "cancelled" ? (
            <span className="badge warning">Stopped</span>
          ) : null}
          {message.interrupted ? (
            <span className="badge warning">Interrupted</span>
          ) : message.status === "error" ? (
            <span className="badge danger">Error</span>
          ) : null}
          {isActiveStream && streaming.attempt ? (
            <span className="faint small-text">
              {streaming.statusLine ?? `Attempt ${streaming.attempt}`}
            </span>
          ) : null}
          <span className="grow" />
          {assistant && content ? (
            <button
              className="icon-btn"
              title="Copy response"
              onClick={() => void copy()}
              style={{ width: 24, height: 24 }}
            >
              {copied ? <Check size={12} /> : <Copy size={12} />}
            </button>
          ) : null}
        </div>

        {message.reasoning ? (
          <details className="reasoning">
            <summary>Reasoning</summary>
            <pre>{message.reasoning}</pre>
          </details>
        ) : null}

        {assistant ? (
          content ? (
            <div className="message-body">
              <Markdown content={content} />
            </div>
          ) : isActiveStream ? (
            <div className="typing" aria-label="Generating">
              <span />
              <span />
              <span />
            </div>
          ) : null
        ) : (
          <div className="message-bubble">{content}</div>
        )}

        {view.attachments.length > 0 ? (
          <div className="msg-attachments">
            {view.attachments.map((attachment) => (
              <AttachmentPreview key={attachment.id} attachment={attachment} />
            ))}
          </div>
        ) : null}

        {assistant && message.segments.length > 0 ? (
          <div className="segment-list">
            {message.segments.map((segment) => {
              const label = accounts.find(
                (account) => account.id === segment.accountId,
              )?.label;
              return (
                <span className="badge segment" key={segment.id}>
                  {segment.model ?? "unknown model"}
                  {label ? ` · ${label}` : ""}
                  {segment.status === "interrupted" ? " · partial" : ""}
                </span>
              );
            })}
          </div>
        ) : null}

        {message.error || message.interrupted ? (
          <div className="message-error">
            <span>
              {message.interrupted
                ? message.interruptionReason ?? message.error ?? "Response interrupted"
                : message.error}
            </span>
            {message.interrupted ? (
              <button
                className="btn small ghost"
                style={{ marginLeft: 8 }}
                disabled={streaming.active}
                onClick={() => void continueResponse(message.id)}
              >
                <RotateCcw size={11} />
                Continue
              </button>
            ) : (
              <button
                className="btn small ghost"
                style={{ marginLeft: 8 }}
                onClick={() => void refresh()}
              >
                <RotateCcw size={11} />
                Refresh
              </button>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}

export function MessageList() {
  const messages = useChatStore((state) => state.messages);
  const streaming = useChatStore((state) => state.streaming);
  const ref = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);

  useEffect(() => {
    const element = ref.current;
    if (!element || !stickToBottom.current) return;
    element.scrollTop = element.scrollHeight;
  }, [messages, streaming.content, streaming.reasoning]);

  return (
    <div
      className="messages"
      ref={ref}
      onScroll={() => {
        const element = ref.current;
        if (!element) return;
        stickToBottom.current =
          element.scrollHeight - element.scrollTop - element.clientHeight < 120;
      }}
    >
      <div className="messages-inner">
        {messages.map((view) => (
          <MessageRow key={view.message.id} view={view} />
        ))}
      </div>
    </div>
  );
}
