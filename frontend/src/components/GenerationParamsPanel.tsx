import {useCallback, useEffect, useState} from "react";
import {fetchJson, type GenerationParams, postJson} from "@/api";
import {cn} from "@/lib/cn";
import {AlertCircle, Check, Droplets, RotateCw, Save, Thermometer} from "lucide-react";

const PARAMS_REFRESH_MS = 5000;

export function GenerationParamsPanel() {
  const [params, setParams] = useState<GenerationParams | null>(null);
  const [temperature, setTemperature] = useState("");
  const [topP, setTopP] = useState("");
  const [saving, setSaving] = useState<"temperature" | "top-p" | null>(null);
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const loadParams = useCallback(async () => {
    try {
      const d = await fetchJson<GenerationParams>("/v1/sys-dashboard/generation-params");
      setParams(d);
      // Only update input fields if user is not currently editing (no save in progress)
      setTemperature(d.temperature?.toString() ?? "");
      setTopP(d.top_p?.toString() ?? "");
    } catch {
      // Non-critical
    }
  }, []);

  useEffect(() => {
    loadParams();
    const id = setInterval(loadParams, PARAMS_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadParams]);

  const handleSaveTemperature = async () => {
    const val = parseFloat(temperature);
    if (isNaN(val) || val < 0 || val > 2) {
      setMsg({text: "Temperature must be between 0.0 and 2.0", ok: false});
      return;
    }
    setSaving("temperature");
    setMsg(null);
    try {
      await postJson("/v1/sys-dashboard/generation-params/temperature", {temperature: val});
      setMsg({text: `Temperature set to ${val}`, ok: true});
      setParams((p) => p ? {...p, temperature: val} : null);
    } catch (e) {
      setMsg({text: e instanceof Error ? e.message : "Failed to set temperature", ok: false});
    } finally {
      setSaving(null);
    }
  };

  const handleSaveTopP = async () => {
    const val = parseFloat(topP);
    if (isNaN(val) || val < 0 || val > 1) {
      setMsg({text: "Top-p must be between 0.0 and 1.0", ok: false});
      return;
    }
    setSaving("top-p");
    setMsg(null);
    try {
      await postJson("/v1/sys-dashboard/generation-params/top-p", {top_p: val});
      setMsg({text: `Top-p set to ${val}`, ok: true});
      setParams((p) => p ? {...p, top_p: val} : null);
    } catch (e) {
      setMsg({text: e instanceof Error ? e.message : "Failed to set top-p", ok: false});
    } finally {
      setSaving(null);
    }
  };

  const handleReset = () => {
    setTemperature(params?.temperature?.toString() ?? "");
    setTopP(params?.top_p?.toString() ?? "");
    setMsg(null);
  };

  return (
    <div className="bg-surface border border-border-subtle rounded-lg overflow-hidden">
      <div className="px-4 py-3 border-b border-border-subtle bg-surface-raised/30">
        <h2 className="text-[13px] font-semibold text-text-sec uppercase tracking-wider flex items-center gap-2">
          <Thermometer size={13}/>
          Generation Parameters
        </h2>
      </div>

      <div className="p-4 space-y-4">
        {/* Temperature */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 w-32 shrink-0">
            <Thermometer size={14} className="text-orange"/>
            <span className="text-[13px] font-medium text-text">Temperature</span>
          </div>
          <div className="flex items-center gap-2 flex-1 max-w-xs">
            <input
              type="number"
              min="0"
              max="2"
              step="0.05"
              value={temperature}
              onChange={(e) => setTemperature(e.target.value)}
              placeholder="0.0 – 2.0"
              className="flex-1 px-3 py-1.5 rounded-md bg-bg border border-border text-text text-[13px] font-mono focus:outline-none focus:border-accent"
            />
            <button
              onClick={handleSaveTemperature}
              disabled={saving === "temperature"}
              className={cn(
                "flex items-center gap-1 px-3 py-1.5 rounded-md text-[12px] font-medium transition-colors cursor-pointer",
                saving === "temperature"
                  ? "bg-accent/50 text-white cursor-wait"
                  : "bg-accent text-white hover:bg-accent-muted"
              )}
            >
              {saving === "temperature" ? (
                <RotateCw size={12} className="animate-spin"/>
              ) : (
                <Save size={12}/>
              )}
              Save
            </button>
          </div>
          {params?.temperature != null && (
            <span className="text-[12px] text-text-tert font-mono">
              Current: {params.temperature}
            </span>
          )}
        </div>

        {/* Top-p */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 w-32 shrink-0">
            <Droplets size={14} className="text-accent"/>
            <span className="text-[13px] font-medium text-text">Top-p</span>
          </div>
          <div className="flex items-center gap-2 flex-1 max-w-xs">
            <input
              type="number"
              min="0"
              max="1"
              step="0.05"
              value={topP}
              onChange={(e) => setTopP(e.target.value)}
              placeholder="0.0 – 1.0"
              className="flex-1 px-3 py-1.5 rounded-md bg-bg border border-border text-text text-[13px] font-mono focus:outline-none focus:border-accent"
            />
            <button
              onClick={handleSaveTopP}
              disabled={saving === "top-p"}
              className={cn(
                "flex items-center gap-1 px-3 py-1.5 rounded-md text-[12px] font-medium transition-colors cursor-pointer",
                saving === "top-p"
                  ? "bg-accent/50 text-white cursor-wait"
                  : "bg-accent text-white hover:bg-accent-muted"
              )}
            >
              {saving === "top-p" ? (
                <RotateCw size={12} className="animate-spin"/>
              ) : (
                <Save size={12}/>
              )}
              Save
            </button>
          </div>
          {params?.top_p != null && (
            <span className="text-[12px] text-text-tert font-mono">
              Current: {params.top_p}
            </span>
          )}
        </div>

        {/* Message bar */}
        {msg && (
          <div
            className={cn(
              "flex items-center gap-2 px-3 py-2 rounded-md text-[12px]",
              msg.ok ? "bg-green-muted text-green" : "bg-red-muted text-red"
            )}
          >
            {msg.ok ? <Check size={13}/> : <AlertCircle size={13}/>}
            {msg.text}
            <button
              onClick={() => setMsg(null)}
              className="ml-auto opacity-60 hover:opacity-100 cursor-pointer"
            >
              ×
            </button>
          </div>
        )}

        {/* Reset button */}
        <div className="flex justify-end">
          <button
            onClick={handleReset}
            className="flex items-center gap-1 px-3 py-1.5 rounded-md text-[12px] text-text-sec hover:bg-surface-raised transition-colors cursor-pointer"
          >
            <RotateCw size={12}/>
            Reset to saved values
          </button>
        </div>
      </div>
    </div>
  );
}
