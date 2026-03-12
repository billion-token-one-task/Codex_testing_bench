import type { PropsWithChildren, ReactNode } from "react";

export function Panel({
  title,
  kicker,
  actions,
  children,
}: PropsWithChildren<{ title: string; kicker?: string; actions?: ReactNode }>) {
  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          {kicker ? <div className="panel-kicker">{kicker}</div> : null}
          <h2>{title}</h2>
        </div>
        {actions ? <div className="panel-actions">{actions}</div> : null}
      </header>
      <div className="panel-body">{children}</div>
    </section>
  );
}
