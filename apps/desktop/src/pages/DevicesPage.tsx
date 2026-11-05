import { useCallback, useState } from "react";
import { api, CATEGORY, STATE, type DeviceState } from "../api";
import type { PageProps } from "../App";
import { useLoad } from "../useLoad";
import { Badge, Empty, PageTitle } from "../ui";

const TONE: Record<DeviceState, "bad" | "warn" | "info" | "good" | "neutral"> = {
  missing_driver: "bad",
  error: "bad",
  updatable: "info",
  ok: "good",
  unknown: "neutral",
};

export default function DevicesPage(_: PageProps) {
  const devices = useLoad(useCallback(() => api.devices(), []));
  const [filter, setFilter] = useState("");
  const rows = (devices.data ?? []).filter((d) => d.name.toLowerCase().includes(filter.toLowerCase()));

  return (
    <>
      <PageTitle
        title="Dispositivos"
        action={
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Buscar dispositivo"
            aria-label="Buscar dispositivo"
            className="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-neutral-500"
          />
        }
      />
      {devices.error && <Empty>{devices.error}</Empty>}
      {devices.data && rows.length === 0 && <Empty>No hay dispositivos que coincidan.</Empty>}
      {rows.length > 0 && (
        <div className="overflow-hidden rounded-2xl border border-white/8">
          <table className="w-full text-left text-sm">
            <thead className="bg-white/5 text-neutral-400">
              <tr>
                <th className="px-4 py-2 font-medium">Dispositivo</th>
                <th className="px-4 py-2 font-medium">Tipo</th>
                <th className="px-4 py-2 font-medium">Estado</th>
                <th className="px-4 py-2 font-medium">Proveedor</th>
                <th className="px-4 py-2 font-medium">Versión</th>
                <th className="px-4 py-2 font-medium">Fecha</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((d) => (
                <tr key={d.instance_id} className="border-t border-white/5">
                  <td className="px-4 py-2 text-white">{d.name}</td>
                  <td className="px-4 py-2 text-neutral-400">{CATEGORY[d.category]}</td>
                  <td className="px-4 py-2">
                    <Badge tone={TONE[d.state]}>{STATE[d.state]}</Badge>
                  </td>
                  <td className="px-4 py-2 text-neutral-400">{d.provider || "-"}</td>
                  <td className="px-4 py-2 font-mono text-xs text-neutral-300">{d.version || "-"}</td>
                  <td className="px-4 py-2 text-neutral-400">{d.date || "-"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
}
