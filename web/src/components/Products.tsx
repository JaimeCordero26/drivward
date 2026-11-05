import { Activity, Check, Cpu, History, MonitorSmartphone, RotateCcw, Usb, Wifi } from "lucide-react";

function WindowBar({ title }: { title: string }) {
  return (
    <div className="flex h-8 items-center justify-between border-b border-white/5 bg-white/[0.03] px-3">
      <span className="text-[11px] font-medium text-neutral-500">{title}</span>
      <div className="flex gap-1.5" aria-hidden="true">
        <span className="h-2 w-2 rounded-full bg-white/10" />
        <span className="h-2 w-2 rounded-full bg-white/10" />
        <span className="h-2 w-2 rounded-full bg-white/10" />
      </div>
    </div>
  );
}

function PortableMockup() {
  const rows = [
    { name: "Wi-Fi · Intel AX201", state: "Instalado", ok: true },
    { name: "Ethernet · Realtek 8168", state: "Instalado", ok: true },
    { name: "Gráfica · sin driver", state: "Pendiente", ok: false },
    { name: "Audio · sin driver", state: "Pendiente", ok: false },
  ];
  return (
    <div
      aria-hidden="true"
      className="relative mt-12 overflow-hidden rounded-xl border border-white/5 bg-[#0f0f0f] shadow-2xl transition-transform duration-500 group-hover:-translate-y-2.5"
    >
      <WindowBar title="Drivward Portable" />
      <div className="p-5">
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-bold text-white">
            <Wifi size={16} className="text-blue-400" /> Conectado a internet
          </div>
          <span className="rounded-md bg-blue-600 px-2.5 py-1 text-[11px] font-bold text-white">Escanear</span>
        </div>
        <ul className="space-y-2">
          {rows.map((row) => (
            <li
              key={row.name}
              className="flex items-center justify-between rounded-lg border border-white/5 bg-white/[0.02] px-3 py-2 text-xs"
            >
              <span className="text-neutral-300">{row.name}</span>
              <span className={row.ok ? "flex items-center gap-1 text-emerald-400" : "text-neutral-500"}>
                {row.ok && <Check size={12} />}
                {row.state}
              </span>
            </li>
          ))}
        </ul>
        <div className="mt-4 h-1.5 overflow-hidden rounded-full bg-white/5">
          <div className="h-full w-1/2 rounded-full bg-blue-500" />
        </div>
      </div>
      <div className="animate-scan absolute inset-x-0 top-8 h-px bg-gradient-to-r from-transparent via-blue-400 to-transparent [--scan-distance:200px]" />
    </div>
  );
}

function DesktopMockup() {
  return (
    <div
      aria-hidden="true"
      className="relative mt-12 overflow-hidden rounded-xl border border-white/10 bg-[#0d0f12] shadow-2xl transition-transform duration-500 group-hover:-translate-y-2.5"
    >
      <WindowBar title="Drivward · Bandeja del sistema" />
      <div className="p-5">
        <div className="mb-4 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500/10">
            <Activity size={18} className="text-emerald-400" />
          </div>
          <div>
            <p className="text-sm font-bold text-white">Equipo estable</p>
            <p className="text-[11px] text-neutral-500">Modo Recomendar · 14 días sin fallos</p>
          </div>
        </div>
        <div className="rounded-lg border border-amber-400/20 bg-amber-400/[0.04] p-3">
          <div className="mb-1 flex items-center gap-2 text-xs font-bold text-amber-200">
            <History size={13} /> Cambio detectado
          </div>
          <p className="mb-3 text-[11px] leading-relaxed text-neutral-400">
            Windows Update cambió el driver de gráfica. Desde entonces hubo 2 fallos de video. Confianza: media.
          </p>
          <div className="flex gap-2">
            <span className="flex items-center gap-1 rounded-md bg-white px-2.5 py-1 text-[11px] font-bold text-black">
              <RotateCcw size={11} /> Volver a la anterior
            </span>
            <span className="rounded-md border border-white/10 px-2.5 py-1 text-[11px] font-bold text-neutral-300">
              Ver evidencia
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function Products() {
  return (
    <section id="versiones" className="mx-auto max-w-7xl scroll-mt-20 px-4 py-24 sm:px-6 md:px-12 md:py-32">
      <div className="reveal mb-14 max-w-3xl">
        <h2 className="text-4xl font-medium leading-[1.05] tracking-tight text-white md:text-6xl">
          Dos versiones, <span className="text-neutral-600">un mismo núcleo.</span>
        </h2>
      </div>
      <div className="grid grid-cols-1 gap-6 md:grid-cols-2 md:gap-8">
        <article className="reveal group relative flex flex-col justify-between overflow-hidden rounded-[2.5rem] border border-[#1a1a1a] bg-[#0a0a0a] p-8 md:p-12">
          <div className="relative z-10">
            <span className="mb-4 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.3em] text-blue-500">
              <Usb size={14} aria-hidden="true" /> Para después de formatear
            </span>
            <h3 className="mb-5 text-5xl font-bold tracking-tighter text-white">Portable</h3>
            <p className="max-w-sm text-lg text-neutral-400">
              Funciona desde USB, sin instalación y sin internet en el primer paso.
            </p>
            <ul className="mt-6 space-y-2 text-sm text-neutral-500">
              <li>Un solo .exe, sin .NET ni WebView2</li>
              <li>Librería de drivers de red verificada con SHA-256</li>
              <li>Si no hay driver compatible, guía para compartir internet desde el celular</li>
            </ul>
          </div>
          <div className="pointer-events-none absolute -bottom-10 -right-10 h-64 w-64 rounded-full bg-blue-600/10 blur-3xl transition-all duration-700 group-hover:bg-blue-600/20" />
          <PortableMockup />
        </article>

        <article className="reveal group relative flex flex-col justify-between overflow-hidden rounded-[2.5rem] border border-[#1a1a1a] bg-gradient-to-br from-[#16181c] to-[#050505] p-8 md:p-12 [transition-delay:80ms]">
          <div className="relative z-10">
            <span className="mb-4 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.3em] text-blue-500">
              <MonitorSmartphone size={14} aria-hidden="true" /> Para el día a día
            </span>
            <h3 className="mb-5 text-5xl font-bold tracking-tighter text-white">Escritorio</h3>
            <p className="max-w-sm text-lg text-neutral-400">
              Vive en la bandeja del sistema, avisa cuando algo cambia y ofrece volver a la versión anterior.
            </p>
            <ul className="mt-6 space-y-2 text-sm text-neutral-500">
              <li>Modos Observar, Recomendar y Automático</li>
              <li>Muestra la evidencia y el nivel de confianza, no supone culpables</li>
              <li>Un solo cambio importante a la vez</li>
            </ul>
          </div>
          <DesktopMockup />
        </article>
      </div>
      <p className="reveal mt-8 flex items-center gap-2 text-sm text-neutral-500">
        <Cpu size={14} aria-hidden="true" /> Ambas usan el mismo núcleo en Rust para detectar, elegir e instalar.
      </p>
    </section>
  );
}
