import {type ReactNode} from "react";
import {cn} from "@/lib/cn";

interface StatCardProps {
  label: string;
  value: string;
  sub?: string;
  icon?: ReactNode;
  accent?: boolean;
  className?: string;
}

export function StatCard({label, value, sub, icon, accent, className}: StatCardProps) {
  return (
    <div
      className={cn(
        "bg-surface border border-border-subtle rounded-lg px-4 py-3.5 transition-colors hover:border-border",
        className,
      )}
    >
      <div className="flex items-center gap-2 mb-1.5">
        {icon && <span className="text-text-tert">{icon}</span>}
        <span className="text-[12px] text-text-sec font-medium uppercase tracking-wider">
          {label}
        </span>
      </div>
      <div className={cn("text-2xl font-bold tabular-nums tracking-tight", accent && "text-accent")}>
        {value}
      </div>
      {sub && <div className="text-[12px] text-text-tert mt-0.5">{sub}</div>}
    </div>
  );
}
