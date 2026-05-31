import {Link, Outlet, useLocation} from "react-router-dom";
import {ActivitySquare, FileText, Moon, Server, Settings, Sun, Wifi, WifiOff} from "lucide-react";
import * as Tooltip from "@radix-ui/react-tooltip";
import {useTheme} from "@/lib/theme";
import {cn} from "@/lib/cn";
import {useEffect, useState} from "react";

export function App() {
  const location = useLocation();
  const {theme, toggle} = useTheme();
  const isTrc = location.pathname === "/trc";
  const isProviders = location.pathname === "/providers";
  const isConfig = location.pathname === "/config";
  const [online, setOnline] = useState(true);

  useEffect(() => {
    const check = async () => {
      try {
        await fetch("/healthz");
        setOnline(true);
      } catch {
        setOnline(false);
      }
    };
    check();
    const id = setInterval(check, 10000);
    return () => clearInterval(id);
  }, []);

  return (
    <Tooltip.Provider delayDuration={200}>
      <div className="min-h-screen bg-bg text-text antialiased text-[14px]">
        <div className="flex min-h-screen">
          {/* Sidebar */}
          <aside className="w-14 bg-surface border-r border-border-subtle flex flex-col items-center py-4 gap-1 shrink-0">
            <div className="w-8 h-8 rounded-lg bg-accent/15 flex items-center justify-center mb-4">
              <ActivitySquare size={16} className="text-accent"/>
            </div>

            <NavItem to="/"
                     icon={<FileText size={18}/>}
                     label="Dashboard"
                     active={!isTrc && !isProviders && !isConfig}/>
            <NavItem to="/providers" icon={<Server size={18}/>} label="Providers" active={isProviders}/>
            <NavItem to="/trc" icon={<ActivitySquare size={18}/>} label="TRC" active={isTrc}/>

            <div className="mt-auto flex flex-col items-center gap-1">
              <button
                onClick={toggle}
                className="w-10 h-10 rounded-lg flex items-center justify-center text-text-tert hover:bg-surface-raised hover:text-text-sec transition-colors cursor-pointer"
                title={theme === "light" ? "Switch to dark" : "Switch to light"}
              >
                {theme === "light" ? <Moon size={18}/> : <Sun size={18}/>}
              </button>
              <NavItem to="/config" icon={<Settings size={18}/>} label="Config" active={isConfig}/>
            </div>
          </aside>

          {/* Main content */}
          <main className="flex-1 overflow-auto">
            <div className="max-w-[1400px] mx-auto px-6 py-5">
              <header className="flex items-center justify-between mb-6">
                <div className="flex items-center gap-3">
                  <h1 className="text-[16px] font-semibold tracking-tight text-text">
                    {isTrc ? "Trace Records" : isProviders ? "Providers" : isConfig ? "Config" : "Codex-Shim Dashboard"}
                  </h1>
                  <span className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-surface-raised text-[12px] text-text-sec font-medium">
                    {online ? (
                      <>
                        <Wifi size={12} className="text-green"/>
                        <span>Connected</span>
                      </>
                    ) : (
                      <>
                        <WifiOff size={12} className="text-red"/>
                        <span className="text-red">Offline</span>
                      </>
                    )}
                  </span>
                </div>
                <div className="flex items-center gap-3 text-[12px] text-text-tert">
                  <span>Auto-refresh 3s</span>
                </div>
              </header>

              <Outlet/>
            </div>
          </main>
        </div>
      </div>
    </Tooltip.Provider>
  );
}

function NavItem({to, icon, label, active}: { to: string; icon: React.ReactNode; label: string; active: boolean }) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <Link
          to={to}
          className={cn(
            "w-10 h-10 rounded-lg flex items-center justify-center transition-colors",
            active
              ? "bg-accent/15 text-accent"
              : "text-text-tert hover:bg-surface-raised hover:text-text-sec",
          )}
        >
          {icon}
        </Link>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side="right"
          sideOffset={8}
          className="px-2.5 py-1 rounded-md bg-surface-raised border border-border text-[12px] text-text-sec shadow-md z-50"
        >
          {label}
          <Tooltip.Arrow className="fill-surface-raised"/>
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}