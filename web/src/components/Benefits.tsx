import { Anchor, BadgeCheck, Code2, EyeOff, Hand, LifeBuoy } from "lucide-react";

const PRINCIPLES = [
  {
    icon: BadgeCheck,
    title: "Drivers oficiales",
    text: "Prioriza drivers oficiales y firmados. Rechaza los que no tienen firma válida o vienen de una fuente no aprobada.",
  },
  {
    icon: Anchor,
    title: "Estable antes que nuevo",
    text: "Elige el driver más estable para tu equipo, no solo el más nuevo, y espera antes de recomendar una versión recién publicada.",
  },
  {
    icon: LifeBuoy,
    title: "Respaldo primero",
    text: "Crea un respaldo de los drivers actuales y un punto de restauración antes de hacer cambios.",
  },
  {
    icon: Hand,
    title: "Tú apruebas",
    text: "Nada se instala sin tu aprobación. Ves qué cambia, de dónde viene y por qué se recomienda.",
  },
  {
    icon: EyeOff,
    title: "Tus datos se quedan",
    text: "No envía datos del equipo sin tu consentimiento. La elección del driver se hace en tu compu.",
  },
  {
    icon: Code2,
    title: "Código abierto",
    text: "Licencia GPL-3.0. Cualquiera puede revisar cómo decide e instala. Sin publicidad ni programas extra.",
  },
];

export default function Benefits() {
  return (
    <section id="principios" className="mx-auto max-w-7xl scroll-mt-20 px-4 py-24 sm:px-6 md:px-12 md:py-32">
      <div className="reveal mb-16 flex items-end justify-between border-b border-[#1f1f1f] pb-10">
        <h2 className="text-xs font-bold uppercase tracking-[0.4em] text-blue-500">Principios</h2>
        <span className="hidden text-xs font-medium uppercase tracking-widest text-neutral-600 md:block">
          No dejar un equipo peor de como lo encontró
        </span>
      </div>

      <div className="grid grid-cols-1 gap-x-12 gap-y-14 sm:grid-cols-2 lg:grid-cols-3">
        {PRINCIPLES.map((item, index) => (
          <div key={item.title} className="reveal group space-y-4" style={{ transitionDelay: `${(index % 3) * 70}ms` }}>
            <div className="inline-flex rounded-xl border border-blue-500/15 bg-blue-500/5 p-3 transition-colors group-hover:border-blue-500/40">
              <item.icon size={24} className="text-blue-500" aria-hidden="true" />
            </div>
            <h3 className="text-2xl font-bold text-white">{item.title}</h3>
            <p className="leading-relaxed text-neutral-500">{item.text}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
