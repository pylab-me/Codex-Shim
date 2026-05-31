import {useCallback, useEffect, useState} from "react";
import {fetchJson, type TrcResponse} from "@/api";
import {fmt} from "@/lib/format";
import {cn} from "@/lib/cn";
import * as ScrollArea from "@radix-ui/react-scroll-area";

const REFRESH_MS = 3000;

export function TrcPage() {
  const [records, setRecords] = useState<TrcResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const d = await fetchJson<TrcResponse>("/v1/sys-dashboard/requests?limit=1000");
      setRecords(d);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load records");
    }
  }, []);

  useEffect(() => {
    load();
    const id = setInterval(load, REFRESH_MS);
    return () => clearInterval(id);
  }, [load]);

  const recs = records?.records || [];

  return (
    <div className="space-y-4">
      {/* Error banner */}
      {error && (
        <div className="flex items-center gap-2 px-4 py-2.5 rounded-lg bg-red-muted border border-red/20 text-red text-[13px]">
          <span>⚠</span>
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            className="ml-auto opacity-60 hover:opacity-100 cursor-pointer"
          >
            ×
          </button>
        </div>
      )}

      <div className="flex items-center gap-4 text-[12px] text-text-tert">
        <span>
          Records:{" "}
          <span className="font-semibold text-text-sec">{recs.length}</span>{" "}
          / 10,000 max
        </span>
      </div>

      <div className="bg-surface border border-border-subtle rounded-lg overflow-hidden">
        <ScrollArea.Root className="max-h-[calc(100vh-200px)]" type="auto">
          <ScrollArea.Viewport className="h-full">
            <table className="w-full border-collapse text-[13px]">
              <thead className="sticky top-0 z-10">
              <tr className="bg-surface-raised border-b border-border">
                <Th right>#</Th>
                <Th>Trace ID</Th>
                <Th>Time</Th>
                <Th>Client Model</Th>
                <Th>Provider Model</Th>
                <Th>Route</Th>
                <Th right>Prompt</Th>
                <Th right>Completion</Th>
                <Th right>Total</Th>
                <Th right>Cached</Th>
                <Th right>Reasoning</Th>
                <Th right>Provider ms</Th>
                <Th right>Wall ms</Th>
                <Th right>Status</Th>
                <Th>Error</Th>
              </tr>
              </thead>
              <tbody>
              {recs.length === 0 ? (
                <tr>
                  <td colSpan={15} className="text-center py-16 text-text-tert text-[13px]">
                    No records yet
                  </td>
                </tr>
              ) : (
                recs.map((r) => {
                  const isErr = r.status_code !== 200;
                  const shortTrace =
                    r.trace_id.length > 16 ? r.trace_id.substring(0, 16) + "..." : r.trace_id;
                  return (
                    <tr
                      key={r.seq}
                      className={cn(
                        "border-b border-border-subtle hover:bg-surface-raised transition-colors",
                        isErr
                          ? "[&>td:first-child]:shadow-[inset_3px_0_0_var(--color-red)]"
                          : "[&>td:first-child]:shadow-[inset_3px_0_0_var(--color-green)]",
                      )}
                    >
                      <td className="text-right px-3 py-2 text-text-tert font-mono text-[12px] tabular-nums whitespace-nowrap">
                        {r.seq}
                      </td>
                      <td className="px-3 py-2 font-mono text-[12px] tabular-nums whitespace-nowrap" title={r.trace_id}>
                        {shortTrace}
                      </td>
                      <td className="px-3 py-2 text-text-sec text-[12px] whitespace-nowrap">
                        {r.created_at}
                      </td>
                      <td className="px-3 py-2 font-medium whitespace-nowrap">{r.client_model || "--"}</td>
                      <td className="px-3 py-2 font-medium whitespace-nowrap">{r.provider_model || "--"}</td>
                      <td className="px-3 py-2 whitespace-nowrap">
                          <span className="inline-block px-1.5 py-0.5 rounded bg-surface-raised text-[11px] text-text-sec font-mono">
                            {r.model_route || ""}
                          </span>
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums whitespace-nowrap">
                        {fmt(r.prompt_tokens)}
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums whitespace-nowrap">
                        {fmt(r.completion_tokens)}
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums font-semibold text-accent whitespace-nowrap">
                        {fmt(r.total_tokens)}
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums text-text-sec whitespace-nowrap">
                        {fmt(r.cached_tokens)}
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums text-text-sec whitespace-nowrap">
                        {fmt(r.reasoning_tokens)}
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums whitespace-nowrap">
                        {r.provider_ms}
                      </td>
                      <td className="text-right px-3 py-2 font-mono text-[12px] tabular-nums whitespace-nowrap">
                        {r.request_time_ms}
                      </td>
                      <td className="text-right px-3 py-2 whitespace-nowrap">
                          <span
                            className={cn(
                              "inline-block px-1.5 py-0.5 rounded text-[11px] font-semibold",
                              isErr
                                ? "bg-red-muted text-red"
                                : "bg-green-muted text-green",
                            )}
                          >
                            {r.status_code}
                          </span>
                      </td>
                      <td className="px-3 py-2 whitespace-nowrap">
                        {r.error && r.error !== "-" ? (
                          <span className="text-red text-[11px] cursor-help" title={r.error}>
                              x
                          </span>
                        ) : null}
                      </td>
                    </tr>
                  );
                })
              )}
              </tbody>
            </table>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar orientation="vertical" className="flex select-none touch-none p-0.5">
            <ScrollArea.Thumb className="flex-1 bg-border rounded-full relative"/>
          </ScrollArea.Scrollbar>
          <ScrollArea.Scrollbar orientation="horizontal" className="flex select-none touch-none p-0.5 h-2">
            <ScrollArea.Thumb className="flex-1 bg-border rounded-full relative"/>
          </ScrollArea.Scrollbar>
        </ScrollArea.Root>
      </div>
    </div>
  );
}

function Th({children, right}: { children: React.ReactNode; right?: boolean }) {
  return (
    <th
      className={cn(
        "px-3 py-2.5 text-[11px] font-medium text-text-sec uppercase tracking-wider whitespace-nowrap",
        right && "text-right",
      )}
    >
      {children}
    </th>
  );
}