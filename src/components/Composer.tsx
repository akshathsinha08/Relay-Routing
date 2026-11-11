import { open } from "@tauri-apps/plugin-dialog";
import { ArrowUp, Paperclip, Square, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { formatBytes } from "@/lib/format";
import { useAppStore } from "@/stores/app";
import { useChatStore } from "@/stores/chat";

export function Composer() {
  const [text, setText] = useState("");
  const attachments = useChatStore((state) => state.attachments);
  const streaming = useChatStore((state) => state.streaming);
  const send = useChatStore((state) => state.send);
  const cancel = useChatStore((state) => state.cancel);
  const attachFiles = useChatStore((state) => state.attachFiles);
  const removeAttachment = useChatStore((state) => state.removeAttachment);
  const sendOnEnter = useAppStore((state) => state.settings.sendOnEnter !== "false");
  const ref = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${Math.min(element.scrollHeight, 220)}px`;
  }, [text]);

  useEffect(() => {
    if (streaming.active) ref.current?.blur();
  }, [streaming.active]);

  const disabled =
    streaming.active || (text.trim().length === 0 && attachments.length === 0);

  const submit = async () => {
    if (disabled) return;
    const value = text;
    setText("");
    const ok = await send(value);
    if (!ok) setText((current) => (current.length === 0 ? value : current));
    ref.current?.focus();
  };

  const pickFiles = async () => {
    try {
      const picked = await open({ multiple: true, title: "Attach files" });
      if (!picked) return;
      const paths = Array.isArray(picked) ? picked : [picked];
      await attachFiles(paths);
    } catch {
      // The dialog is optional; dismissal is not an error worth surfacing.
    }
  };

  return (
    <div className="composer-wrap">
      <div className="composer">
        {attachments.length > 0 ? (
          <div className="composer-attachments">
            {attachments.map((attachment) => (
              <span className="attach-chip" key={attachment.id}>
                <Paperclip size={11} />
                <span className="name">{attachment.name}</span>
                <span className="faint">{formatBytes(attachment.size)}</span>
                <button
                  onClick={() => void removeAttachment(attachment.id)}
                  aria-label={`Remove ${attachment.name}`}
                >
                  <X size={11} />
                </button>
              </span>
            ))}
          </div>
        ) : null}
        <textarea
          ref={ref}
          rows={1}
          value={text}
          placeholder="Message Relay Routing…"
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              if (event.shiftKey) return;
              const wantsSend = sendOnEnter ? !event.metaKey : event.metaKey;
              if (wantsSend) {
                event.preventDefault();
                void submit();
              }
            }
          }}
          aria-label="Message"
        />
        <div className="composer-bar">
          <button
            className="icon-btn"
            onClick={() => void pickFiles()}
            title="Attach files"
            disabled={streaming.active}
          >
            <Paperclip size={15} />
          </button>
          <span className="grow" />
          {streaming.active ? (
            <button
              className="send stop"
              onClick={() => void cancel()}
              title="Stop generating"
            >
              <Square size={13} fill="currentColor" />
            </button>
          ) : (
            <button
              className="send"
              onClick={() => void submit()}
              disabled={disabled}
              title="Send"
            >
              <ArrowUp size={16} />
            </button>
          )}
        </div>
      </div>
      <div className="composer-hint">
        <span>
          {sendOnEnter ? "Enter to send · Shift+Enter for a new line" : "⌘Enter to send"}
        </span>
        <span>Local-first · data stays on this Mac</span>
      </div>
    </div>
  );
}
