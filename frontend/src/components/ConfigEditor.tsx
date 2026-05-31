import {useCallback, useState} from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import * as ScrollArea from "@radix-ui/react-scroll-area";
import {ChevronRight, FileCode, RotateCw, Save} from "lucide-react";
import {type ConfigContent, fetchJson, putJson} from "@/api";
import {cn} from "@/lib/cn";

export function ConfigEditor() {
  const [open, setOpen] = useState(false);
  const [content, setContent] = useState("");
  const [filename, setFilename] = useState("--");
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const loadConfig = useCallback(async () => {
    try {
      const d = await fetchJson<ConfigContent>("/v1/sys-dashboard/config");
      setContent(d.content);
      if (d.path) setFilename(d.path);
      setMsg({text: "Loaded", ok: true});
    } catch {
      setContent("# config not found");
      setMsg({text: "Not found", ok: false});
    }
  }, []);

  const saveConfig = useCallback(async () => {
    try {
      const d = await putJson<{ checksum: string }>("/v1/sys-dashboard/config", {content});
      setMsg({text: "Saved. " + (d.checksum || ""), ok: true});
    } catch (e) {
      setMsg({text: (e as Error).message, ok: false});
    }
  }, [content]);

  return (
    <Collapsible.Root
      open={open}
      onOpenChange={(v) => {
        setOpen(v);
        if (v && content === "") loadConfig();
      }}
    >
      <div className="bg-surface border border-border-subtle rounded-lg overflow-hidden">
        <Collapsible.Trigger asChild>
          <button className="w-full flex items-center justify-between px-4 py-3 cursor-pointer select-none hover:bg-surface-raised transition-colors">
            <div className="flex items-center gap-2.5">
              <FileCode size={15} className="text-text-tert"/>
              <span className="text-[14px] font-medium">Config Editor</span>
              <span className="text-[12px] text-text-tert font-mono">{filename}</span>
            </div>
            <ChevronRight
              size={14}
              className={cn(
                "text-text-tert transition-transform duration-200",
                open && "rotate-90",
              )}
            />
          </button>
        </Collapsible.Trigger>

        <Collapsible.Content>
          <div className="border-t border-border-subtle">
            <ScrollArea.Root className="h-[220px]" type="auto">
              <ScrollArea.Viewport className="h-full">
                <textarea
                  className="w-full h-[220px] bg-bg text-text border-none px-4 py-3 font-mono text-[13px] leading-relaxed resize-none outline-none"
                  value={content}
                  onChange={(e) => setContent(e.target.value)}
                  placeholder="Loading..."
                />
              </ScrollArea.Viewport>
              <ScrollArea.Scrollbar orientation="vertical" className="flex select-none touch-none p-0.5">
                <ScrollArea.Thumb className="flex-1 bg-border rounded-full relative"/>
              </ScrollArea.Scrollbar>
            </ScrollArea.Root>

            <div className="flex items-center gap-2 px-4 py-2.5 border-t border-border-subtle bg-surface-raised/30">
              <button
                onClick={loadConfig}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[12px] font-medium bg-surface-raised border border-border text-text-sec hover:bg-surface-hover hover:text-text transition-colors cursor-pointer"
              >
                <RotateCw size={12}/>
                Reload
              </button>
              <button
                onClick={saveConfig}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[12px] font-medium bg-accent text-white hover:bg-accent-muted transition-colors cursor-pointer"
              >
                <Save size={12}/>
                Save
              </button>
              {msg && (
                <span className={cn("text-[12px] ml-2", msg.ok ? "text-green" : "text-red")}>
                  {msg.text}
                </span>
              )}
            </div>
          </div>
        </Collapsible.Content>
      </div>
    </Collapsible.Root>
  );
}
