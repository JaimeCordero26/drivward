import { useCallback } from "react";
import { api, CATEGORY } from "../api";
import type { PageProps } from "../App";
import { useLoad } from "../useLoad";
import { Badge, Button, Card, Empty, PageTitle } from "../ui";

export default function RecommendationsPage({ canControl, run }: PageProps) {
  const list = useLoad(useCallback(() => api.recommendations(), []));
  const settings = useLoad(useCallback(() => api.settings(), []));
  const observe = settings.data?.mode === "observe";

  return (
    <>
      <PageTitle
        title="Recomendaciones"
        text="Cada recomendación explica de dónde viene y por qué. Se aplica un cambio importante a la vez y antes se guarda una copia del driver actual."
      />
      {observe && (
        <Card className="mb-4 text-sm text-neutral-300">
          Estás en modo Observar: Drivward no instala nada. Cambia a Recomendar en Ajustes para aprobar instalaciones.
        </Card>
      )}
      {list.error && <Empty>{list.error}</Empty>}
      {list.data?.length === 0 && <Empty>No hay actualizaciones recomendadas. Tus drivers están al día.</Empty>}
      <div className="space-y-3">
        {list.data?.map((r) => {
          const waiting = Boolean(r.available_after);
          return (
            <Card key={r.id}>
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <p className="font-semibold text-white">{r.device_name}</p>
                <Badge>{CATEGORY[r.category]}</Badge>
                {r.requires_approval && <Badge tone="info">Requiere aprobación</Badge>}
                {waiting && <Badge tone="warn">En espera hasta {r.available_after}</Badge>}
              </div>
              <p className="mb-2 text-sm text-neutral-400">
                Actual: {r.current_version ?? "-"} · Nueva: {r.new_version ?? "-"} · Origen: {r.source}
              </p>
              <ul className="mb-3 list-disc space-y-1 pl-5 text-sm text-neutral-300">
                {r.reasons.map((reason) => (
                  <li key={reason}>{reason}</li>
                ))}
              </ul>
              {r.blocked_reason && <p className="mb-3 text-sm text-amber-200">{r.blocked_reason}</p>}
              <div className="flex flex-wrap gap-2">
                <Button
                  kind="primary"
                  disabled={!canControl || observe || waiting || Boolean(r.blocked_reason)}
                  onClick={async () => {
                    await run(() => api.act({ type: "install", recommendation_id: r.id }));
                    list.reload();
                  }}
                >
                  Aprobar e instalar
                </Button>
                <Button
                  disabled={!canControl}
                  onClick={async () => {
                    await run(() => api.act({ type: "block", update_id: r.id }));
                    list.reload();
                  }}
                >
                  No instalar esta versión
                </Button>
              </div>
            </Card>
          );
        })}
      </div>
    </>
  );
}
