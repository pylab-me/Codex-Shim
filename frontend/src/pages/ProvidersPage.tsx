import {useCallback, useEffect, useState} from "react";
import {fetchJson, postJson, type ProvidersResponse, type ProviderStatus,} from "@/api";
import {CheckCircle2, ChevronDown, Key, Pencil, Plus, Save, Server, Shield, Trash2, X, XCircle,} from "lucide-react";
import * as Dialog from "@radix-ui/react-dialog";
import {cn} from "@/lib/cn";

export function ProvidersPage() {
  const [data, setData] = useState<ProvidersResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Import key dialog
  const [importOpen, setImportOpen] = useState(false);
  const [importProvider, setImportProvider] = useState("");
  const [importKey, setImportKey] = useState("");

  // Add provider dialog
  const [addOpen, setAddOpen] = useState(false);
  const [addName, setAddName] = useState("");
  const [addUrl, setAddUrl] = useState("");
  const [addModel, setAddModel] = useState("");
  const [addModels, setAddModels] = useState("");
  const [addThinking, setAddThinking] = useState(false);
  const [addTemp, setAddTemp] = useState("");
  const [addTopP, setAddTopP] = useState("");
  const [addMaxTokens, setAddMaxTokens] = useState("");

  // Edit provider dialog
  const [editOpen, setEditOpen] = useState(false);
  const [editName, setEditName] = useState("");
  const [editUrl, setEditUrl] = useState("");
  const [editModel, setEditModel] = useState("");
  const [editModels, setEditModels] = useState("");
  const [editThinking, setEditThinking] = useState(false);
  const [editTemp, setEditTemp] = useState("");
  const [editTopP, setEditTopP] = useState("");
  const [editMaxTokens, setEditMaxTokens] = useState("");

  const load = useCallback(async () => {
    try {
      const d = await fetchJson<ProvidersResponse>("/v1/sys-dashboard/providers");
      setData(d);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load providers");
    }
  }, []);

  // Fetch once on mount (route navigation into this page triggers mount).
  useEffect(() => {
    load();
  }, [load]);

  const clearMsg = () => {
    setError(null);
    setSuccess(null);
  };

  const handleImport = async () => {
    if (!importProvider.trim() || !importKey.trim()) {
      setError("Provider name and API key are required.");
      return;
    }
    setLoading(true);
    clearMsg();
    try {
      const res = await postJson<{ key_hash: string }>(
        "/v1/sys-dashboard/providers/import-key",
        {provider: importProvider.trim(), api_key: importKey.trim()},
      );
      setSuccess(`Key imported for "${importProvider}" (hash: ${res.key_hash})`);
      setImportProvider("");
      setImportKey("");
      setImportOpen(false);
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Import failed");
    } finally {
      setLoading(false);
    }
  };

  const handleSwitch = async (provider: string, model: string) => {
    setLoading(true);
    clearMsg();
    try {
      await postJson("/v1/sys-dashboard/providers/switch", {provider, model});
      setSuccess(`Switched to "${provider}" / "${model}"`);
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Switch failed");
    } finally {
      setLoading(false);
    }
  };

  const handleRemoveKey = async (provider: string) => {
    setLoading(true);
    clearMsg();
    try {
      await postJson("/v1/sys-dashboard/providers/remove-key", {name: provider});
      setSuccess(`Key removed for "${provider}"`);
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Remove failed");
    } finally {
      setLoading(false);
    }
  };

  const handleAdd = async () => {
    if (!addName.trim() || !addUrl.trim() || !addModel.trim()) {
      setError("Name, URL, and default model are required.");
      return;
    }
    setLoading(true);
    clearMsg();
    try {
      const models = addModels
        ? addModels
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean)
        : [];
      await postJson("/v1/sys-dashboard/providers/add", {
        name: addName.trim(),
        base_url: addUrl.trim(),
        model: addModel.trim(),
        models,
        thinking: addThinking,
        default_temperature: addTemp ? parseFloat(addTemp) : undefined,
        default_top_p: addTopP ? parseFloat(addTopP) : undefined,
        default_max_output_tokens: addMaxTokens ? parseInt(addMaxTokens) : undefined,
      });
      setSuccess(`Provider "${addName}" added`);
      setAddOpen(false);
      setAddName("");
      setAddUrl("");
      setAddModel("");
      setAddModels("");
      setAddThinking(false);
      setAddTemp("");
      setAddTopP("");
      setAddMaxTokens("");
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Add failed");
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async () => {
    if (!editName) return;
    setLoading(true);
    clearMsg();
    try {
      const models = editModels
        ? editModels
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean)
        : undefined;
      await postJson("/v1/sys-dashboard/providers/update", {
        name: editName,
        base_url: editUrl || undefined,
        model: editModel || undefined,
        models,
        thinking: editThinking,
        default_temperature: editTemp ? parseFloat(editTemp) : undefined,
        default_top_p: editTopP ? parseFloat(editTopP) : undefined,
        default_max_output_tokens: editMaxTokens ? parseInt(editMaxTokens) : undefined,
      });
      setSuccess(`Provider "${editName}" updated`);
      setEditOpen(false);
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Update failed");
    } finally {
      setLoading(false);
    }
  };

  const handleRemove = async (name: string) => {
    setLoading(true);
    clearMsg();
    try {
      await postJson("/v1/sys-dashboard/providers/remove", {name});
      setSuccess(`Provider "${name}" removed`);
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Remove failed");
    } finally {
      setLoading(false);
    }
  };

  const handleSaveConfig = async () => {
    setLoading(true);
    clearMsg();
    try {
      const res = await postJson<{ checksum: string }>(
        "/v1/sys-dashboard/providers/save-config",
        {},
      );
      setSuccess(`Config saved (${res.checksum})`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setLoading(false);
    }
  };

  const openEdit = (p: ProviderStatus) => {
    setEditName(p.name);
    setEditUrl(p.base_url);
    setEditModel(p.model);
    setEditModels(p.models.join(", "));
    setEditThinking(p.thinking);
    setEditTemp(p.default_temperature?.toString() ?? "");
    setEditTopP(p.default_top_p?.toString() ?? "");
    setEditMaxTokens(p.default_max_output_tokens?.toString() ?? "");
    setEditOpen(true);
  };

  const providers = data?.providers ?? [];
  const activeProvider = data?.active_provider ?? "--";
  const activeModel = data?.active_model ?? "--";

  return (
    <div className="space-y-4">
      {/* Messages */}
      {error && <MsgBar kind="error" text={error} onClose={clearMsg}/>}
      {success && (
        <MsgBar kind="success" text={success} onClose={clearMsg}/>
      )}

      {/* Active profile banner */}
      <div className="flex items-center gap-3 px-4 py-2.5 rounded-lg bg-accent/8 border border-accent/15">
        <CheckCircle2 size={15} className="text-accent shrink-0"/>
        <span className="text-[13px] text-text">
          Active:{" "}
          <strong className="text-accent">{activeProvider}</strong>
          {" / "}
          <code className="font-mono text-accent">{activeModel}</code>
        </span>
      </div>

      {/* Provider table */}
      <section className="bg-surface border border-border-subtle rounded-lg overflow-hidden">
        {/* Table header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border-subtle bg-surface-raised/30">
          <h2 className="text-[12px] font-semibold text-text-sec uppercase tracking-wider flex items-center gap-2">
            <Server size={13}/> Providers
          </h2>
          <div className="flex gap-2">
            <Btn onClick={handleSaveConfig} disabled={loading} subtle>
              <Save size={12}/> Save config
            </Btn>
            <Btn onClick={() => setImportOpen(true)} disabled={loading} subtle>
              <Key size={12}/> Import key
            </Btn>
            <Btn onClick={() => setAddOpen(true)} subtle>
              <Plus size={12}/> Add provider
            </Btn>
          </div>
        </div>

        {/* Column headers */}
        <div className="grid grid-cols-[48px_1fr_180px_100px_72px] gap-0 px-4 py-2 border-b border-border-subtle bg-surface-raised/10 text-[11px] font-medium text-text-tert uppercase tracking-wider">
          <div className="text-center">Active</div>
          <div>Provider</div>
          <div>Model</div>
          <div className="text-center">Key</div>
          <div className="text-center">Actions</div>
        </div>

        {/* Rows */}
        {providers.length === 0 && (
          <div className="px-4 py-8 text-center text-text-tert text-[13px]">
            No providers configured. Click "Add provider" to get started.
          </div>
        )}
        {providers.map((p) => (
          <ProviderRow
            key={p.name}
            provider={p}
            isActive={p.name === activeProvider && activeModel !== "--"}
            activeModel={activeModel}
            onSwitch={handleSwitch}
            onRemoveKey={handleRemoveKey}
            onRemove={handleRemove}
            onEdit={openEdit}
            loading={loading}
          />
        ))}
      </section>

      {/* Security note */}
      <div className="flex items-start gap-2 px-4 py-3 rounded-lg bg-surface-raised/50 border border-border-subtle text-[12px] text-text-tert">
        <Shield size={13} className="shrink-0 mt-0.5"/>
        <span>
          API keys are stored securely in the OS and are never written to disk.
        </span>
      </div>

      {/* ── Dialogs ── */}

      {/* Import Key Dialog */}
      <Dialog.Root open={importOpen} onOpenChange={setImportOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/20 z-50"/>
          <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-surface border border-border-subtle rounded-xl shadow-xl w-[420px] p-0 z-50 focus:outline-none">
            <div className="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
              <Dialog.Title className="text-[14px] font-semibold text-text">
                Import API Key
              </Dialog.Title>
              <Dialog.Close className="text-text-tert hover:text-text transition-colors cursor-pointer">
                <X size={16}/>
              </Dialog.Close>
            </div>
            <div className="p-5 space-y-4">
              <div>
                <Label>Provider</Label>
                <select
                  value={importProvider}
                  onChange={(e) => setImportProvider(e.target.value)}
                  className={inputCls}
                >
                  <option value="">Select provider...</option>
                  {providers.map((p) => (
                    <option key={p.name} value={p.name}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <Label>API Key</Label>
                <input
                  type="password"
                  value={importKey}
                  onChange={(e) => setImportKey(e.target.value)}
                  placeholder="sk-..."
                  className={inputCls + " font-mono"}
                />
              </div>
              <div className="flex justify-end gap-2 pt-1">
                <Btn onClick={() => setImportOpen(false)} subtle>
                  Cancel
                </Btn>
                <Btn
                  onClick={handleImport}
                  disabled={loading || !importProvider || !importKey}
                >
                  Import
                </Btn>
              </div>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      {/* Add Provider Dialog */}
      <Dialog.Root open={addOpen} onOpenChange={setAddOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/20 z-50"/>
          <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-surface border border-border-subtle rounded-xl shadow-xl w-[520px] p-0 z-50 focus:outline-none">
            <div className="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
              <Dialog.Title className="text-[14px] font-semibold text-text">
                Add Provider
              </Dialog.Title>
              <Dialog.Close className="text-text-tert hover:text-text transition-colors cursor-pointer">
                <X size={16}/>
              </Dialog.Close>
            </div>
            <div className="p-5 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label>Name</Label>
                  <input
                    value={addName}
                    onChange={(e) => setAddName(e.target.value)}
                    placeholder="ollama"
                    className={inputCls}
                  />
                </div>
                <div>
                  <Label>Base URL</Label>
                  <input
                    value={addUrl}
                    onChange={(e) => setAddUrl(e.target.value)}
                    placeholder="http://127.0.0.1:11434/v1"
                    className={inputCls}
                  />
                </div>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label>Default model</Label>
                  <input
                    value={addModel}
                    onChange={(e) => setAddModel(e.target.value)}
                    placeholder="qwen3:8b"
                    className={inputCls}
                  />
                </div>
                <div>
                  <Label>Models (comma separated)</Label>
                  <input
                    value={addModels}
                    onChange={(e) => setAddModels(e.target.value)}
                    placeholder="qwen3:8b, qwen3:14b"
                    className={inputCls}
                  />
                </div>
              </div>
              <label className="flex items-center gap-1.5 text-[13px] text-text-sec cursor-pointer">
                <input
                  type="checkbox"
                  checked={addThinking}
                  onChange={(e) => setAddThinking(e.target.checked)}
                  className="accent-accent"
                />
                Thinking mode
              </label>
              {/* Generation Parameters */}
              <div className="pt-2 border-t border-border-subtle">
                <p className="text-[12px] font-medium text-text-sec mb-2">Generation Parameters</p>
                <div className="grid grid-cols-3 gap-3">
                  <div>
                    <Label>Temperature</Label>
                    <input
                      type="number"
                      min="0"
                      max="2"
                      step="0.05"
                      value={addTemp}
                      onChange={(e) => setAddTemp(e.target.value)}
                      placeholder="0.0 – 2.0"
                      className={inputCls + " font-mono"}
                    />
                  </div>
                  <div>
                    <Label>Top-p</Label>
                    <input
                      type="number"
                      min="0"
                      max="1"
                      step="0.05"
                      value={addTopP}
                      onChange={(e) => setAddTopP(e.target.value)}
                      placeholder="0.0 – 1.0"
                      className={inputCls + " font-mono"}
                    />
                  </div>
                  <div>
                    <Label>Max tokens</Label>
                    <input
                      type="number"
                      min="1"
                      value={addMaxTokens}
                      onChange={(e) => setAddMaxTokens(e.target.value)}
                      placeholder="e.g. 4096"
                      className={inputCls + " font-mono"}
                    />
                  </div>
                </div>
              </div>
              <div className="flex justify-end gap-2 pt-1">
                <Btn onClick={() => setAddOpen(false)} subtle>
                  Cancel
                </Btn>
                <Btn
                  onClick={handleAdd}
                  disabled={loading || !addName || !addUrl || !addModel}
                >
                  Add provider
                </Btn>
              </div>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      {/* Edit Provider Dialog */}
      <Dialog.Root open={editOpen} onOpenChange={setEditOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/20 z-50"/>
          <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-surface border border-border-subtle rounded-xl shadow-xl w-[520px] p-0 z-50 focus:outline-none">
            <div className="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
              <Dialog.Title className="text-[14px] font-semibold text-text">
                Edit: {editName}
              </Dialog.Title>
              <Dialog.Close className="text-text-tert hover:text-text transition-colors cursor-pointer">
                <X size={16}/>
              </Dialog.Close>
            </div>
            <div className="p-5 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label>Base URL</Label>
                  <input
                    value={editUrl}
                    onChange={(e) => setEditUrl(e.target.value)}
                    className={inputCls}
                  />
                </div>
                <div>
                  <Label>Default model</Label>
                  <input
                    value={editModel}
                    onChange={(e) => setEditModel(e.target.value)}
                    className={inputCls}
                  />
                </div>
              </div>
              <div>
                <Label>Models (comma separated)</Label>
                <input
                  value={editModels}
                  onChange={(e) => setEditModels(e.target.value)}
                  className={inputCls}
                />
              </div>
              <label className="flex items-center gap-1.5 text-[13px] text-text-sec cursor-pointer">
                <input
                  type="checkbox"
                  checked={editThinking}
                  onChange={(e) => setEditThinking(e.target.checked)}
                  className="accent-accent"
                />
                Thinking mode
              </label>
              {/* Generation Parameters */}
              <div className="pt-2 border-t border-border-subtle">
                <p className="text-[12px] font-medium text-text-sec mb-2">Generation Parameters</p>
                <div className="grid grid-cols-3 gap-3">
                  <div>
                    <Label>Temperature</Label>
                    <input
                      type="number"
                      min="0"
                      max="2"
                      step="0.05"
                      value={editTemp}
                      onChange={(e) => setEditTemp(e.target.value)}
                      placeholder="0.0 – 2.0"
                      className={inputCls + " font-mono"}
                    />
                  </div>
                  <div>
                    <Label>Top-p</Label>
                    <input
                      type="number"
                      min="0"
                      max="1"
                      step="0.05"
                      value={editTopP}
                      onChange={(e) => setEditTopP(e.target.value)}
                      placeholder="0.0 – 1.0"
                      className={inputCls + " font-mono"}
                    />
                  </div>
                  <div>
                    <Label>Max tokens</Label>
                    <input
                      type="number"
                      min="1"
                      value={editMaxTokens}
                      onChange={(e) => setEditMaxTokens(e.target.value)}
                      placeholder="e.g. 4096"
                      className={inputCls + " font-mono"}
                    />
                  </div>
                </div>
              </div>
              <div className="flex justify-end gap-2 pt-1">
                <Btn onClick={() => setEditOpen(false)} subtle>
                  Cancel
                </Btn>
                <Btn onClick={handleUpdate} disabled={loading}>
                  Save changes
                </Btn>
              </div>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  );
}

// ── Provider Row ──

function ProviderRow({
                       provider,
                       isActive,
                       activeModel,
                       onSwitch,
                       onRemoveKey,
                       onRemove,
                       onEdit,
                       loading,
                     }: {
  provider: ProviderStatus;
  isActive: boolean;
  activeModel: string;
  onSwitch: (provider: string, model: string) => void;
  onRemoveKey: (provider: string) => void;
  onRemove: (name: string) => void;
  onEdit: (p: ProviderStatus) => void;
  loading: boolean;
}) {
  const [selModel, setSelModel] = useState(provider.model);

  useEffect(() => {
    setSelModel(isActive ? activeModel : provider.model);
  }, [provider.model, isActive, activeModel]);

  const modelChanged = isActive && selModel !== activeModel;
  const canActivate = !isActive && provider.has_key;
  const needsSwitch = modelChanged && provider.has_key;

  return (
    <div
      className={cn(
        "grid grid-cols-[48px_1fr_180px_100px_72px] gap-0 items-center px-4 py-3 border-b border-border-subtle last:border-b-0 transition-colors",
        isActive && "bg-accent/5",
      )}
    >
      {/* Active checkbox */}
      <div className="flex justify-center">
        <button
          onClick={() => {
            if (canActivate) onSwitch(provider.name, selModel);
            else if (needsSwitch) onSwitch(provider.name, selModel);
          }}
          disabled={loading || (!canActivate && !needsSwitch)}
          className={cn(
            "w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all cursor-pointer",
            isActive
              ? "bg-accent border-accent text-white"
              : canActivate
                ? "border-border hover:border-accent/50 hover:bg-accent/5"
                : "border-border-subtle opacity-40 cursor-not-allowed",
          )}
          title={
            isActive
              ? "Currently active"
              : canActivate
                ? "Click to activate"
                : "Import a key first"
          }
        >
          {isActive && <CheckCircle2 size={13} strokeWidth={3}/>}
        </button>
      </div>

      {/* Provider info */}
      <div className="min-w-0 pr-3">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-medium text-text truncate">
            {provider.name}
          </span>
          {provider.thinking && (
            <span className="px-1.5 py-0.5 rounded bg-orange-muted text-orange text-[10px] font-medium shrink-0">
              thinking
            </span>
          )}
        </div>
        <div className="text-[12px] text-text-tert mt-0.5 truncate">
          {provider.base_url}
        </div>
      </div>

      {/* Model selector */}
      <div className="flex items-center justify-center">
        <div className="relative w-full">
          <select
            value={selModel}
            onChange={(e) => setSelModel(e.target.value)}
            className={cn(
              "w-full px-2.5 py-1.5 pr-7 rounded-md bg-bg border text-[12px] text-text font-mono focus:outline-none focus:ring-1 focus:ring-accent/30 appearance-none cursor-pointer",
              isActive ? "border-accent/30" : "border-border",
            )}
          >
            {provider.models.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
          <ChevronDown
            size={12}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-text-tert pointer-events-none"
          />
        </div>
      </div>

      {/* Key status */}
      <div className="flex justify-center">
        {provider.has_key ? (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-green-muted text-green text-[12px] font-medium">
            <Key size={10}/> Set
          </span>
        ) : (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-red-muted text-red text-[12px] font-medium">
            <XCircle size={10}/> None
          </span>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center justify-center gap-0.5">
        <button
          onClick={() => onEdit(provider)}
          disabled={loading}
          className="w-8 h-8 rounded-md flex items-center justify-center text-text-tert hover:bg-surface-raised hover:text-text-sec transition-colors disabled:opacity-40 cursor-pointer"
          title="Edit provider"
        >
          <Pencil size={13}/>
        </button>
        <button
          onClick={() =>
            provider.has_key
              ? onRemoveKey(provider.name)
              : onRemove(provider.name)
          }
          disabled={loading || (isActive && !provider.has_key)}
          className="w-8 h-8 rounded-md flex items-center justify-center text-text-tert hover:bg-red-muted hover:text-red transition-colors disabled:opacity-40 cursor-pointer"
          title={provider.has_key ? "Remove key" : "Remove provider"}
        >
          <Trash2 size={13}/>
        </button>
      </div>
    </div>
  );
}

// ── Shared UI ──

function MsgBar({
                  kind,
                  text,
                  onClose,
                }: {
  kind: "error" | "success";
  text: string;
  onClose: () => void;
}) {
  const cls =
    kind === "error"
      ? "bg-red-muted border-red/20 text-red"
      : "bg-green-muted border-green/20 text-green";
  const Icon = kind === "error" ? XCircle : CheckCircle2;
  return (
    <div
      className={`flex items-center gap-2 px-4 py-2.5 rounded-lg border text-[13px] ${cls}`}
    >
      <Icon size={14}/> {text}
      <button
        onClick={onClose}
        className="ml-auto opacity-60 hover:opacity-100 cursor-pointer"
      >
        &times;
      </button>
    </div>
  );
}

function Label({children}: { children: React.ReactNode }) {
  return (
    <label className="block text-[12px] text-text-tert mb-1">{children}</label>
  );
}

const inputCls =
  "w-full px-3 py-2 rounded-md bg-bg border border-border text-text text-[13px] focus:outline-none focus:border-accent";

function Btn({
               children,
               onClick,
               disabled,
               subtle,
             }: {
  children: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
  subtle?: boolean;
}) {
  const cls = subtle
    ? "flex items-center gap-1 px-3 py-2 rounded-md text-[13px] text-text-sec hover:bg-surface-raised transition-colors disabled:opacity-40 cursor-pointer"
    : "flex items-center gap-1 px-4 py-2 rounded-md bg-accent text-white text-[13px] font-medium hover:bg-accent-muted transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer";
  return (
    <button onClick={onClick} disabled={disabled} className={cls}>
      {children}
    </button>
  );
}
