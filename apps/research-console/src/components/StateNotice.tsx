import type { ReactNode } from "react";

type Tone = "loading" | "info" | "warning" | "error" | "success";

export function StateNotice({
  title,
  body,
  tone = "info",
  children,
}: {
  title: string;
  body?: ReactNode;
  tone?: Tone;
  children?: ReactNode;
}) {
  return (
    <div className={`state-notice state-notice-${tone}`}>
      <div className="state-notice-header">
        <span className="state-notice-chip">{tone}</span>
        <strong>{title}</strong>
      </div>
      {body ? <div className="state-notice-body">{body}</div> : null}
      {children ? <div className="state-notice-actions">{children}</div> : null}
    </div>
  );
}
