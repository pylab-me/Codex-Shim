import type {ModelStatsEntry} from "@/api";
import {fmt, fmtMs, fmtRound} from "@/lib/format";
import {cn} from "@/lib/cn";

interface ModelTableProps {
  models: ModelStatsEntry[];
}

export function ModelTable({models}: ModelTableProps) {
  if (models.length === 0) {
    return (
      <div className="text-center py-12 text-text-tert text-[13px]">
        No model requests yet
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[13px]">
        <thead>
        <tr className="border-b border-border">
          <Th>Model</Th>
          <Th>Route</Th>
          <Th align="right">Requests</Th>
          <Th align="right">Prompt</Th>
          <Th align="right">Completion</Th>
          <Th align="right">Total</Th>
          <Th align="right">Cached</Th>
          <Th align="right">Reasoning</Th>
          <Th align="right">Avg Provider</Th>
          <Th align="right">Avg Wall</Th>
          <Th align="right">Avg Prompt</Th>
          <Th align="right">Status</Th>
        </tr>
        </thead>
        <tbody>
        {models.map((m, i) => {
          const rc = m.request_count || 1;
          return (
            <tr
              key={i}
              className="border-b border-border-subtle hover:bg-surface-raised transition-colors"
            >
              <Td className="font-medium text-text">{m.provider_model}</Td>
              <Td>
                  <span className="inline-block px-1.5 py-0.5 rounded bg-surface-raised text-[11px] text-text-sec font-mono">
                    {m.model_route}
                  </span>
              </Td>
              <Td align="right" mono>
                {m.request_count}
                {m.error_count > 0 && (
                  <span className="text-red text-[11px] ml-1">({m.error_count}e)</span>
                )}
              </Td>
              <Td align="right" mono>{fmt(m.prompt_tokens)}</Td>
              <Td align="right" mono>{fmt(m.completion_tokens)}</Td>
              <Td align="right" mono className="font-semibold text-accent">
                {fmt(m.total_tokens)}
              </Td>
              <Td align="right" mono className="text-text-sec">
                {fmt(m.cached_tokens)}
              </Td>
              <Td align="right" mono className="text-text-sec">
                {fmt(m.reasoning_tokens)}
              </Td>
              <Td align="right" mono>{fmtMs(m.total_provider_ms / rc)}</Td>
              <Td align="right" mono>{fmtMs(m.total_total_ms / rc)}</Td>
              <Td align="right" mono>{fmtRound(m.prompt_tokens / rc)}</Td>
              <Td align="right">
                <StatusBadge status={m.last_status}/>
              </Td>
            </tr>
          );
        })}
        </tbody>
      </table>
    </div>
  );
}

function Th({children, align}: { children: React.ReactNode; align?: "left" | "right" }) {
  return (
    <th
      className={cn(
        "px-3 py-2.5 text-[12px] font-medium text-text-sec uppercase tracking-wider whitespace-nowrap",
        align === "right" ? "text-right" : "text-left",
      )}
    >
      {children}
    </th>
  );
}

function Td({
              children,
              align,
              mono,
              className,
            }: {
  children: React.ReactNode;
  align?: "left" | "right";
  mono?: boolean;
  className?: string;
}) {
  return (
    <td
      className={cn(
        "px-3 py-2.5 whitespace-nowrap",
        mono && "font-mono tabular-nums text-[12px]",
        align === "right" && "text-right",
        className,
      )}
    >
      {children}
    </td>
  );
}

function StatusBadge({status}: { status: number }) {
  const ok = status === 200;
  return (
    <span
      className={cn(
        "inline-block px-1.5 py-0.5 rounded text-[11px] font-semibold",
        ok ? "bg-green-muted text-green" : "bg-red-muted text-red",
      )}
    >
      {status}
    </span>
  );
}
