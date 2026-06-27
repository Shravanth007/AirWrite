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
    <div className="mb-8">
      {(Icon || eyebrow) && (
        <div className="flex items-center gap-1.5 mb-3 text-[11px] text-zinc-500">
          {Icon && (
            <Icon className="w-3.5 h-3.5 text-zinc-500" strokeWidth={2} />
          )}
          {eyebrow && (
            <span className="font-medium tracking-tight">{eyebrow}</span>
          )}
        </div>
      )}
      <h1 className="text-[30px] font-semibold tracking-tight text-white leading-[1.1]">
        {title}
      </h1>
      {description && (
        <p className="text-[13.5px] text-zinc-500 mt-2.5 leading-relaxed max-w-[480px]">
          {description}
        </p>
      )}
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
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label className="text-[10.5px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
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

export function Block({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`border-t border-white/[0.06] pt-6 ${className}`}
    >
      {children}
    </div>
  );
}

export function Pill({
  tone = "neutral",
  children,
}: {
  tone?: "neutral" | "ok" | "warn" | "soon" | "danger";
  children: ReactNode;
}) {
  const styles = {
    neutral: "bg-white/[0.04] text-zinc-400 border-white/[0.08]",
    ok: "bg-white/[0.06] text-white border-white/15",
    warn: "bg-white/[0.04] text-zinc-300 border-white/15",
    soon: "bg-white/[0.04] text-zinc-500 border-white/[0.08]",
    danger: "bg-red-500/10 text-red-300 border-red-500/30",
  } as const;
  return (
    <span
      className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md border text-[9.5px] font-semibold uppercase tracking-[0.12em] ${styles[tone]}`}
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
    "inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-1 focus-visible:ring-white/40 active:scale-[0.97]";
  const sizes = {
    sm: "px-2.5 py-1.5 text-[11.5px]",
    md: "px-3.5 py-2 text-[12.5px]",
  };
  const variants = {
    primary:
      "bg-white text-black hover:bg-zinc-200 active:bg-zinc-300 shadow-[0_0_0_1px_rgba(255,255,255,0.1),0_8px_22px_-8px_rgba(255,255,255,0.25)]",
    secondary:
      "bg-transparent text-zinc-200 border border-white/[0.08] hover:border-white/20 hover:bg-white/[0.03]",
    ghost: "text-zinc-400 hover:text-white hover:bg-white/[0.04]",
    danger:
      "bg-transparent text-red-300 border border-red-500/30 hover:bg-red-500/10 hover:border-red-500/50",
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
      className="w-full bg-transparent border border-white/[0.08] rounded-lg px-3.5 py-2.5 text-[13px] text-zinc-100 placeholder:text-zinc-700 focus:outline-none focus:border-white/30 transition-colors"
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
    <div className="relative">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
        className="w-full appearance-none bg-transparent border border-white/[0.08] rounded-lg px-3.5 py-2.5 pr-10 text-[13px] text-zinc-100 focus:outline-none focus:border-white/30 transition-colors cursor-pointer hover:border-white/15"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value} className="bg-black">
            {o.label}
          </option>
        ))}
      </select>
      <svg
        className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 w-3 h-3 text-zinc-500"
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
