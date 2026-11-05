import { useCallback } from "react";
import { api, CATEGORY, formatDate, type Origin } from "../api";
import type { PageProps } from "../App";
import { useLoad } from "../useLoad";
import { Badge, Button, Card, Empty, PageTitle } from "../ui";

const ORIGIN: Record<Origin, string> = {
  drivward: "Drivward",
  external: "Windows Update u otro programa",
  rollback: "Rollback",
};

export default function HistoryPage({ canControl, run }: PageProps) {
  const history = useLoad(useCallback(() => api.history(200), []));

  return (
    <>
      <PageTitle
        title="Historial"
        text="Todos los cambios de drivers, incluidos los que hizo Windows Update. Cada uno conserva una ruta para volver atrás."
      />
      {history.error && <Empty>{history.error}</Empty>}
      {history.data?.length === 0 && <Empty>Todavía no hay cambios registrados.</Empty>}
      <div className="space-y-3">
        {history.data?.map((c) => (
          <Card key={c.id}>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="mb-1 flex flex-wrap items-center gap-2">
                  <p className="font-semibold text-white">{c.device_name}</p>
                  <Badge>{CATEGORY[c.category]}</Badge>
                  <Badge tone={c.validated ? "good" : "warn"}>{c.validated ? "Validado" : "En observación"}</Badge>
                </div>
                <p className="text-sm text-neutral-400">
                  {formatDate(c.at)} · {c.old_version ?? "sin versión"} → {c.new_version ?? "sin driver"} · {ORIGIN[c.origin]}
                </p>
              </div>
              <div className="flex gap-2">
                {!c.validated && (
                  <Button
                    disabled={!canControl}
                    onClick={async () => {
                      await run(() => api.act({ type: "mark_good", change_id: c.id }));
                      history.reload();
                    }}
                  >
                    Funciona bien
                  </Button>
                )}
                <Button
                  kind="danger"
                  disabled={!canControl || !c.can_rollback}
                  onClick={async () => {
                    await run(() => api.act({ type: "rollback", change_id: c.id }));
                    history.reload();
                  }}
                >
                  Volver a la versión anterior
                </Button>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </>
  );
}
