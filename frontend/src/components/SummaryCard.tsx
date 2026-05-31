interface SummaryCardProps {
  label: string;
  value: string;
  sub?: string;
  accent?: boolean;
}

export function SummaryCard({label, value, sub, accent}: SummaryCardProps) {
  return (
    <div className="bg-surface border border-border-subtle rounded-md px-4 py-3.5 shadow-[0_1px_3px_rgba(0,0,0,0.04)] hover:shadow-[0_2px_8px_rgba(0,0,0,0.06)] hover:-translate-y-px transition-all duration-150">
      <div className="text-[12px] text-text-sec font-medium mb-1">{label}</div>
      <div
        className={`text-2xl font-bold tracking-tight tabular-nums leading-tight ${
          accent ? "text-blue" : ""
        }`}
      >
        {value}
      </div>
      {sub && <div className="text-[12px] text-text-tert mt-0.5">{sub}</div>}
    </div>
  );
}
