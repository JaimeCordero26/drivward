import { useEffect, useState } from "react";
import { Activity, Ban, Cpu, History, Settings as SettingsIcon, ShieldAlert, Sparkles } from "lucide-react";
import { api } from "./api";
import { Button } from "./ui";
import StatusPage from "./pages/StatusPage";
import RecommendationsPage from "./pages/RecommendationsPage";
import HistoryPage from "./pages/HistoryPage";
import DevicesPage from "./pages/DevicesPage";
import BlocksPage from "./pages/BlocksPage";
import SettingsPage from "./pages/SettingsPage";

const PAGES = [
  { id: "status", label: "Estado", icon: Activity },
  { id: "recommendations", label: "Recomendaciones", icon: Sparkles },
  { id: "history", label: "Historial", icon: History },
  { id: "devices", label: "Dispositivos", icon: Cpu },
  { id: "blocks", label: "Bloqueos", icon: Ban },
  { id: "settings", label: "Ajustes", icon: SettingsIcon },
] as const;

export type PageId = (typeof PAGES)[number]["id"];

export interface PageProps {
  canControl: boolean;
  /** Ejecuta una acción y muestra su resultado. */
  run: (action: () => Promise<string>) => Promise<boolean>;
  go: (page: PageId) => void;
}

export default function App() {
  const [page, setPage] = useState<PageId>("status");
  const [canControl, setCanControl] = useState(false);
  const [message, setMessage] = useState<{ text: string; ok: boolean } | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.canControl().then(setCanControl).catch(() => setCanControl(false));
  }, []);

  const run = async (action: () => Promise<string>) => {
    setBusy(true);
    setMessage(null);
    try {
      setMessage({ text: await action(), ok: true });
      return true;
    } catch (e) {
      setMessage({ text: e instanceof Error ? e.message : String(e), ok: false });
      return false;
    } finally {
      setBusy(false);
    }
  };

  const props: PageProps = { canControl, run, go: setPage };

  return (
    <div className="flex h-full">
      <nav aria-label="Secciones" className="flex w-56 shrink-0 flex-col border-r border-white/5 bg-black/30 p-4">
        <div className="mb-8 flex items-center gap-3 px-2">
          <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-600 font-black text-white">D.</span>
          <span className="font-bold text-white">Drivward</span>
        </div>
        <ul className="space-y-1">
          {PAGES.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => setPage(item.id)}
                aria-current={page === item.id ? "page" : undefined}
                className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm font-medium transition-colors ${
                  page === item.id ? "bg-white/10 text-white" : "text-neutral-400 hover:bg-white/5 hover:text-white"
                }`}
              >
                <item.icon size={16} aria-hidden="true" />
                {item.label}
              </button>
            </li>
          ))}
        </ul>
        {!canControl && (
          <div className="mt-auto rounded-xl border border-amber-400/20 bg-amber-400/5 p-3 text-xs text-amber-100">
            <ShieldAlert size={14} className="mb-1" aria-hidden="true" />
            Puedes ver el estado. Para instalar, volver atrás o cambiar ajustes, abre como administrador.
            <div className="mt-2">
              <Button onClick={() => api.relaunchElevated()}>Abrir como administrador</Button>
            </div>
          </div>
        )}
      </nav>

      <main className="flex-1 overflow-y-auto p-8">
        {message && (
          <div
            role="status"
            className={`mb-6 rounded-xl border px-4 py-3 text-sm ${
              message.ok ? "border-emerald-500/20 bg-emerald-500/5 text-emerald-200" : "border-red-500/20 bg-red-500/5 text-red-200"
            }`}
          >
            {message.text}
          </div>
        )}
        {busy && <p className="mb-4 text-sm text-neutral-400">Trabajando…</p>}
        {page === "status" && <StatusPage {...props} />}
        {page === "recommendations" && <RecommendationsPage {...props} />}
        {page === "history" && <HistoryPage {...props} />}
        {page === "devices" && <DevicesPage {...props} />}
        {page === "blocks" && <BlocksPage {...props} />}
        {page === "settings" && <SettingsPage {...props} />}
      </main>
    </div>
  );
}
