import type { ReactNode } from "react";

export function PageIntro({
  title,
  kicker,
  description,
  actions,
}: {
  title: string;
  kicker: string;
  description: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <section className="page-intro">
      {/* Crosshatch texture decoration */}
      <div className="page-intro-texture" aria-hidden="true" />
      <div className="page-intro-copy">
        <div className="page-kicker">{kicker}</div>
        <h2 className="page-title">{title}</h2>
        <p className="page-description">{description}</p>
      </div>
      {actions ? <div className="page-intro-actions">{actions}</div> : null}
    </section>
  );
}
