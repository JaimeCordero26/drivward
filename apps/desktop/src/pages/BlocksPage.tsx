import { useCallback } from "react";
import { api, formatDate } from "../api";
import type { PageProps } from "../App";
import { useLoad } from "../useLoad";
import { Button, Card, Empty, PageTitle } from "../ui";

export default function BlocksPage({ canControl, run }: PageProps) {
  const blocks = useLoad(useCallback(() => api.blocks(), []));

  return (
    <>
      <PageTitle
        title="Bloqueos"
        text="Versiones que pediste no instalar. Windows Update las oculta y no las vuelve a ofrecer hasta que quites el bloqueo."
      />
      {blocks.error && <Empty>{blocks.error}</Empty>}
      {blocks.data?.length === 0 && <Empty>No hay versiones bloqueadas.</Empty>}
      <div className="space-y-3">
        {blocks.data?.map((b) => (
          <Card key={b.update_id}>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="font-semibold text-white">{b.title}</p>
                <p className="text-sm text-neutral-400">
                  Bloqueado el {formatDate(b.created_at)}
                  {b.hardware_id && ` · ${b.hardware_id}`}
                </p>
              </div>
              <Button
                disabled={!canControl}
                onClick={async () => {
                  await run(() => api.act({ type: "unblock", update_id: b.update_id }));
                  blocks.reload();
                }}
              >
                Quitar bloqueo
              </Button>
            </div>
          </Card>
        ))}
      </div>
    </>
  );
}
