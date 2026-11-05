import { useCallback, useEffect, useState } from "react";
import { api, MODE, type Mode, type Settings } from "../api";
import type { PageProps } from "../App";
import { useLoad } from "../useLoad";
import { Button, Card, Empty, PageTitle } from "../ui";

function NumberField({
  label,
  help,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  help: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="block">
      <span className="text-sm font-semibold text-white">{label}</span>
      <span className="block text-xs text-neutral-400">{help}</span>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Math.min(max, Math.max(min, Number(e.target.value) || min)))}
        className="mt-2 w-28 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white"
      />
    </label>
  );
}

export default function SettingsPage({ canControl, run }: PageProps) {
  const loaded = useLoad(useCallback(() => api.settings(), []));
  const [settings, setSettings] = useState<Settings | null>(null);
  useEffect(() => setSettings(loaded.data), [loaded.data]);

  if (loaded.error) return <Empty>{loaded.error}</Empty>;
  if (!settings) return <p className="text-sm text-neutral-400">Cargando…</p>;
  const set = (patch: Partial<Settings>) => setSettings({ ...settings, ...patch });
  const paused = settings.paused_until && new Date(settings.paused_until) > new Date();

  return (
    <>
      <PageTitle title="Ajustes" />
      <fieldset className="mb-6" disabled={!canControl}>
        <legend className="mb-3 text-lg font-bold text-white">Modo</legend>
        <div className="grid gap-3 md:grid-cols-3">
          {(Object.keys(MODE) as Mode[]).map((mode) => (
            <label
              key={mode}
              className={`cursor-pointer rounded-2xl border p-4 transition-colors ${
                settings.mode === mode ? "border-blue-500/60 bg-blue-500/10" : "border-white/8 bg-white/[0.03] hover:border-white/20"
              }`}
            >
              <input
                type="radio"
                name="mode"
                className="sr-only"
                checked={settings.mode === mode}
                onChange={() => set({ mode })}
              />
              <span className="block font-semibold text-white">{MODE[mode].label}</span>
              <span className="text-sm text-neutral-400">{MODE[mode].text}</span>
            </label>
          ))}
        </div>
      </fieldset>

      <Card className="mb-6">
        <fieldset disabled={!canControl} className="grid gap-6 md:grid-cols-3">
          <NumberField
            label="Días de espera"
            help="Antes de recomendar una versión recién publicada."
            value={settings.quarantine_days}
            min={0}
            max={90}
            onChange={(quarantine_days) => set({ quarantine_days })}
          />
          <NumberField
            label="Días de observación"
            help="Sin fallos para dar por buena una actualización."
            value={settings.observation_days}
            min={1}
            max={60}
            onChange={(observation_days) => set({ observation_days })}
          />
          <NumberField
            label="Hora de búsqueda"
            help="Hora del día para buscar actualizaciones (0 a 23)."
            value={settings.check_hour}
            min={0}
            max={23}
            onChange={(check_hour) => set({ check_hour })}
          />
          <label className="flex items-start gap-3 md:col-span-3">
            <input
              type="checkbox"
              checked={settings.auto_rollback}
              onChange={(e) => set({ auto_rollback: e.target.checked })}
              className="mt-1"
            />
            <span>
              <span className="block text-sm font-semibold text-white">Rollback automático con evidencia alta</span>
              <span className="text-xs text-neutral-400">
                Si un driver nuevo coincide con varios fallos del mismo tipo, Drivward vuelve solo a la versión anterior y te avisa.
                Apagado por defecto.
              </span>
            </span>
          </label>
        </fieldset>
      </Card>

      <div className="flex flex-wrap gap-2">
        <Button kind="primary" disabled={!canControl} onClick={() => run(() => api.act({ type: "set_settings", settings }))}>
          Guardar
        </Button>
        {paused ? (
          <Button disabled={!canControl} onClick={async () => { await run(() => api.act({ type: "resume" })); loaded.reload(); }}>
            Reanudar monitoreo
          </Button>
        ) : (
          <Button disabled={!canControl} onClick={async () => { await run(() => api.act({ type: "pause", hours: 24 })); loaded.reload(); }}>
            Pausar 24 horas
          </Button>
        )}
      </div>
      <p className="mt-6 max-w-2xl text-xs text-neutral-500">
        Drivward no envía datos del equipo. La elección de drivers se hace en tu compu y los registros se guardan en
        %ProgramData%\Drivward.
      </p>
    </>
  );
}
