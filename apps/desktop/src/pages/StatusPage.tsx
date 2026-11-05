import { useCallback } from "react";
import { RefreshCw } from "lucide-react";
import { api, CONFIDENCE, MODE, formatDate, type Health } from "../api";
import type { PageProps } from "../App";
import { useLoad } from "../useLoad";
import { Badge, Button, Card, Empty, PageTitle } from "../ui";

const HEALTH: Record<Health, { label: string; tone: "good" | "warn" | "bad" | "neutral" }> = {
  stable: { label: "Estable", tone: "good" },
  attention: { label: "Requiere atención", tone: "warn" },
  unstable: { label: "Inestable", tone: "bad" },
  paused: { label: "En pausa", tone: "neutral" },
};

export default function StatusPage({ canControl, run, go }: PageProps) {
  const status = useLoad(useCallback(() => api.status(), []), 30_000);
  const evidence = useLoad(useCallback(() => api.evidence(), []));

  if (status.error) return <Empty>{status.error}</Empty>;
  const s = status.data;
  if (!s) return <p className="text-sm text-neutral-400">Cargando…</p>;
  const health = HEALTH[s.health];
  const suggestions = (evidence.data ?? []).filter((e) => e.confidence !== "none");

  return (
    <>
      <PageTitle
        title="Estado del equipo"
        text={`Última revisión: ${formatDate(s.last_scan)} · Modo ${MODE[s.mode].label}`}
        action={
          <Button
            disabled={!canControl}
            onClick={async () => {
              await run(() => api.act({ type: "scan_now" }));
              status.reload();
              evidence.reload();
            }}
          >
            <RefreshCw size={14} aria-hidden="true" /> Revisar ahora
          </Button>
        }
      />
      <Card className="mb-6">
        <div className="flex items-center gap-3">
          <Badge tone={health.tone}>{health.label}</Badge>
          <p className="text-lg font-semibold text-white">{s.message}</p>
        </div>
        {s.paused_until && <p className="mt-2 text-sm text-neutral-400">En pausa hasta {formatDate(s.paused_until)}.</p>}
      </Card>

      <div className="mb-8 grid grid-cols-2 gap-4 lg:grid-cols-4">
        {[
          { label: "Dispositivos", value: s.devices_total, page: "devices" as const },
          { label: "Con problemas", value: s.devices_attention, page: "devices" as const },
          { label: "Recomendaciones", value: s.recommendations, page: "recommendations" as const },
          { label: "Fallos en 7 días", value: s.events_last_7_days, page: "history" as const },
        ].map((item) => (
          <button key={item.label} type="button" onClick={() => go(item.page)} className="text-left">
            <Card className="transition-colors hover:border-blue-500/30">
              <p className="text-3xl font-bold text-white">{item.value}</p>
              <p className="text-sm text-neutral-400">{item.label}</p>
            </Card>
          </button>
        ))}
      </div>

      <h2 className="mb-3 text-lg font-bold text-white">Cambios recientes y estabilidad</h2>
      {suggestions.length === 0 ? (
        <Empty>No hay fallos relacionados con cambios recientes de drivers.</Empty>
      ) : (
        <div className="space-y-3">
          {suggestions.map((e) => (
            <Card key={e.change_id}>
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <p className="font-semibold text-white">{e.device_name}</p>
                <Badge tone={e.confidence === "high" ? "bad" : e.confidence === "medium" ? "warn" : "neutral"}>
                  Confianza {CONFIDENCE[e.confidence].toLowerCase()}
                </Badge>
              </div>
              <p className="text-sm text-neutral-300">{e.explanation}</p>
              {e.suggest_rollback && (
                <div className="mt-3">
                  <Button
                    kind="danger"
                    disabled={!canControl}
                    onClick={async () => {
                      await run(() => api.act({ type: "rollback", change_id: e.change_id }));
                      status.reload();
                      evidence.reload();
                    }}
                  >
                    Volver a la versión anterior
                  </Button>
                </div>
              )}
            </Card>
          ))}
        </div>
      )}
      <p className="mt-8 text-xs text-neutral-500">Drivward {s.version} · GPL-3.0</p>
    </>
  );
}
