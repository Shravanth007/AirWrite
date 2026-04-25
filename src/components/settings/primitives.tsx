import type { ReactNode } from "react";

export function PageHero({
  eyebrow,
  title,
  description,
  Icon,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  Icon?: typeof import("lucide-react").Mic2;
}) {
  return (
    <div className="relative mb-10">
      <div className="absolute -top-12 -left-8 w-64 h-32 bg-brand-500/10 blur-3xl rounded-full pointer-events-none" />
      <div className="relative">
        {Icon && (
          <div className="mb-4 inline-flex w-10 h-10 rounded-xl bg-gradient-to-br from-[var(--color-surface-3)] to-[var(--color-surface-2)] border border-[var(--color-line)] items-center justify-center shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
            <Icon className="w-4 h-4 text-brand-400" strokeWidth={2} />
          </div>
        )}
        {eyebrow && (
          <div className="text-[10.5px] font-medium uppercase tracking-[0.14em] text-brand-400 mb-1.5">
            {eyebrow}
          </div>
        )}
        <h1 className="text-[22px] font-semibold tracking-tight text-zinc-50 leading-tight">
          {title}
        </h1>
        {description && (
          <p className="text-[13px] text-zinc-500 mt-2 leading-relaxed max-w-[420px]">
            {description}
          </p>
        )}
      </div>
    </div>
  );
}

export function Field({
  label,
  hint,
  children,
  trailing,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  trailing?: ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <label className="text-[10.5px] font-semibold uppercase tracking-[0.12em] text-zinc-400">
          {label}
        </label>
        {trailing}
      </div>
      {children}
      {hint && (
        <p className="text-[11.5px] text-zinc-500 leading-relaxed">{hint}</p>
      )}
    </div>
  );
}

export function Card({
  children,
  className = "",
  glow = false,
}: {
  children: ReactNode;
  className?: string;
  glow?: boolean;
}) {
  return (
    <div
      className={`relative bg-gradient-to-b from-[var(--color-surface-2)] to-[var(--color-surface)] border border-[var(--color-line)] rounded-2xl shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] ${
        glow
          ? "before:absolute before:inset-0 before:rounded-2xl before:bg-gradient-to-br before:from-brand-500/[0.06] before:to-transparent before:pointer-events-none"
          : ""
      } ${className}`}
    >
      <div className="relative">{children}</div>
    </div>
  );
}

export function Pill({
  tone = "neutral",
  children,
}: {
  tone?: "neutral" | "brand" | "warn" | "soon" | "ok";
  children: ReactNode;
}) {
  const styles = {
    neutral: "bg-zinc-900 text-zinc-400 border-zinc-800",
    brand: "bg-brand-500/10 text-brand-300 border-brand-500/30",
    warn: "bg-amber-500/10 text-amber-300 border-amber-500/30",
    soon: "bg-fuchsia-500/10 text-fuchsia-300 border-fuchsia-500/30",
    ok: "bg-emerald-500/10 text-emerald-300 border-emerald-500/30",
  } as const;
  return (
    <span
      className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md border text-[9.5px] font-semibold uppercase tracking-[0.1em] ${styles[tone]}`}
    >
      {children}
    </span>
  );
}

interface ButtonProps {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
  onClick?: () => void;
  disabled?: boolean;
  children: ReactNode;
  type?: "button" | "submit";
  className?: string;
}

export function Button({
  variant = "secondary",
  size = "md",
  onClick,
  disabled,
  children,
  type = "button",
  className = "",
}: ButtonProps) {
  const base =
    "inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500/40 active:scale-[0.97]";
  const sizes = {
    sm: "px-2.5 py-1.5 text-[11.5px]",
    md: "px-3.5 py-2 text-[12.5px]",
  };
  const variants = {
    primary:
      "bg-gradient-to-b from-brand-400 to-brand-500 text-zinc-950 hover:brightness-110 shadow-[0_1px_0_rgba(255,255,255,0.25)_inset,0_8px_22px_-8px_rgba(34,211,238,0.55)]",
    secondary:
      "bg-[var(--color-surface-3)] text-zinc-100 border border-[var(--color-line)] hover:border-[var(--color-line-strong)] hover:bg-[var(--color-surface-2)]",
    ghost:
      "text-zinc-400 hover:text-zinc-100 hover:bg-[var(--color-surface-2)]",
    danger:
      "bg-red-500/10 text-red-300 border border-red-500/30 hover:bg-red-500/20",
  };
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={`${base} ${sizes[size]} ${variants[variant]} ${className}`}
    >
      {children}
    </button>
  );
}

export function TextInput({
  type = "text",
  value,
  onChange,
  placeholder,
  ...rest
}: {
  type?: "text" | "password";
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
} & Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange" | "value" | "type">) {
  return (
    <input
      {...rest}
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className="w-full bg-black border border-[var(--color-line)] rounded-lg px-3.5 py-2.5 text-[13px] text-zinc-100 placeholder:text-zinc-700 focus:outline-none focus:border-brand-500/60 focus:ring-2 focus:ring-brand-500/15 transition-all"
    />
  );
}

export function Select<T extends string>({
  value,
  onChange,
  options,
}: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: string }[];
}) {
  return (
    <div className="relative group">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
        className="w-full appearance-none bg-black border border-[var(--color-line)] rounded-lg px-3.5 py-2.5 pr-10 text-[13px] text-zinc-100 focus:outline-none focus:border-brand-500/60 focus:ring-2 focus:ring-brand-500/15 transition-all cursor-pointer hover:border-[var(--color-line-strong)]"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      <svg
        className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 w-3 h-3 text-zinc-500 group-hover:text-zinc-300 transition-colors"
        viewBox="0 0 12 12"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M3 5l3 3 3-3" />
      </svg>
    </div>
  );
}

export function Divider() {
  return <div className="h-px bg-[var(--color-line)] my-6" />;
}
