import type { ReactNode } from "react";

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-white/8 bg-white/[0.03] p-5 dark:border-white/8 ${className}`}>{children}</div>
  );
}

export function Button({
  children,
  onClick,
  kind = "secondary",
  disabled,
}: {
  children: ReactNode;
  onClick: () => void;
  kind?: "primary" | "secondary" | "danger";
  disabled?: boolean;
}) {
  const style = {
    primary: "bg-blue-600 text-white hover:bg-blue-500",
    secondary: "border border-white/10 bg-white/5 text-neutral-100 hover:bg-white/10",
    danger: "bg-white text-black hover:bg-neutral-200",
  }[kind];
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`inline-flex items-center gap-2 rounded-lg px-3.5 py-2 text-sm font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${style}`}
    >
      {children}
    </button>
  );
}

export function Badge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "good" | "warn" | "bad" | "info" }) {
  const style = {
    neutral: "bg-white/8 text-neutral-300",
    good: "bg-emerald-500/15 text-emerald-300",
    warn: "bg-amber-400/15 text-amber-200",
    bad: "bg-red-500/15 text-red-300",
    info: "bg-blue-500/15 text-blue-300",
  }[tone];
  return <span className={`inline-flex rounded-md px-2 py-0.5 text-xs font-semibold ${style}`}>{children}</span>;
}

export function Empty({ children }: { children: ReactNode }) {
  return <p className="rounded-2xl border border-dashed border-white/10 p-8 text-center text-sm text-neutral-400">{children}</p>;
}

export function PageTitle({ title, text, action }: { title: string; text?: string; action?: ReactNode }) {
  return (
    <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-white">{title}</h1>
        {text && <p className="mt-1 max-w-2xl text-sm text-neutral-400">{text}</p>}
      </div>
      {action}
    </div>
  );
}
