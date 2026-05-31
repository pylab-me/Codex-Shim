import {useCallback, useEffect, useState} from "react";
import {fetchJson, type ModelStatsResponse, type VersionInfo,} from "@/api";
import {fmt, fmtMs} from "@/lib/format";
import {StatCard} from "@/components/StatCard";
import {ModelTable} from "@/components/ModelTable";
import {ArrowUpDown, BarChart3, Brain, Clock, Database, HardDrive, Hash, Zap,} from "lucide-react";

const REFRESH_MS = 3000;

export function DashboardPage() {
  const [stats, setStats] = useState<ModelStatsResponse | null>(null);
  const [version, setVersion] = useState<VersionInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const d = await fetchJson<ModelStatsResponse>("/v1/sys-dashboard/model-stats");
      setStats(d);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load stats");
    }
  }, []);

  const loadVersion = useCallback(async () => {
    try {
      const d = await fetchJson<VersionInfo>("/v1/sys-dashboard/version");
      setVersion(d);
      const el = document.getElementById("ver");
      if (el) el.textContent = "v" + d.version;
    } catch {
      // Version load failure is non-critical
    }
  }, []);

  useEffect(() => {
    load();
    loadVersion();
    const id = setInterval(load, REFRESH_MS);
    return () => clearInterval(id);
  }, [load, loadVersion]);

  const s = stats?.summary;
  const tr = s?.total_requests || 0;

  return (
    <div className="space-y-5">
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

      {/* Row 1: Primary stats */}
      <div className="grid grid-cols-4 gap-3">
        <StatCard
          label="Requests"
          value={fmt(tr)}
          icon={<BarChart3 size={13}/>}
          accent
        />
        <StatCard
          label="Total Tokens"
          value={fmt(s?.total_tokens)}
          icon={<Hash size={13}/>}
          accent
        />
        <StatCard
          label="Avg Provider"
          value={fmtMs(tr > 0 ? (s?.total_provider_ms || 0) / tr : null)}
          sub="latency"
          icon={<Zap size={13}/>}
        />
        <StatCard
          label="Avg Wall"
          value={fmtMs(tr > 0 ? (s?.total_total_ms || 0) / tr : null)}
          sub="time"
          icon={<Clock size={13}/>}
        />
      </div>

      {/* Row 2: Token breakdown */}
      <div className="grid grid-cols-4 gap-3">
        <StatCard
          label="Prompt"
          value={fmt(s?.total_prompt_tokens)}
          icon={<ArrowUpDown size={13}/>}
        />
        <StatCard
          label="Completion"
          value={fmt(s?.total_completion_tokens)}
          icon={<Database size={13}/>}
        />
        <StatCard
          label="Cached"
          value={fmt(s?.total_cached_tokens)}
          icon={<HardDrive size={13}/>}
        />
        <StatCard
          label="Reasoning"
          value={fmt(s?.total_reasoning_tokens)}
          icon={<Brain size={13}/>}
        />
      </div>

      {/* Per-Model Stats */}
      <section id="models">
        <div className="bg-surface border border-border-subtle rounded-lg overflow-hidden">
          <div className="px-4 py-3 border-b border-border-subtle bg-surface-raised/30">
            <h2 className="text-[13px] font-semibold text-text-sec uppercase tracking-wider">
              Per-Model Stats
            </h2>
          </div>
          <ModelTable models={stats?.models || []}/>
        </div>
      </section>

      {/* Footer */}
      <footer className="flex justify-between py-3 border-t border-border-subtle text-[12px] text-text-tert">
        <span>
          Codex-Shim {version ? `v${version.version}` : "--"} | Powered By{' '}
          <a
            href="https://github.com/pylab-me/codex-mimo-shim"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:underline color-primary" // 可选：增加鼠标悬浮下划线效果
          >
            https://github.com/pylab-me/codex-mimo-shim
          </a>
        </span>
        <span className="font-mono">{version?.config_source ?? "--"}</span>
      </footer>
    </div>
  );
}