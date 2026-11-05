import { CheckCheck, HeartPulse, Plug, ScanSearch } from "lucide-react";
import SectionLabel from "./SectionLabel";

const STEPS = [
  {
    icon: Plug,
    title: "Conecta",
    text: "La USB portable instala el driver de red para recuperar internet, sin instalar nada en la compu.",
  },
  {
    icon: ScanSearch,
    title: "Escanea",
    text: "Revisa qué falta y qué se puede actualizar. Sin alarmas ni publicidad.",
  },
  {
    icon: CheckCheck,
    title: "Aprueba",
    text: "Ves qué se va a instalar, de dónde viene cada driver y por qué se recomienda.",
  },
  {
    icon: HeartPulse,
    title: "Cuida",
    text: "La versión de escritorio vigila la estabilidad y permite volver a la versión anterior.",
  },
];

export default function HowItWorks() {
  return (
    <section id="como-funciona" className="mx-auto max-w-7xl scroll-mt-20 px-4 py-24 sm:px-6 md:px-12 md:py-32">
      <div className="grid grid-cols-1 items-start gap-16 lg:grid-cols-2 lg:gap-24">
        <div className="reveal lg:sticky lg:top-32">
          <SectionLabel>Cómo funciona</SectionLabel>
          <h2 className="mb-10 text-4xl font-medium leading-[1.05] tracking-tight text-white md:text-6xl">
            Formatear sin ir <span className="text-neutral-600">driver por driver.</span>
          </h2>
          <p className="mb-6 text-lg leading-relaxed text-neutral-400">
            Después de formatear, instalar cada driver a mano es lento. A veces ni siquiera hay internet para
            descargarlos, y una actualización automática puede dejar el equipo inestable.
          </p>
          <p className="text-lg leading-relaxed text-neutral-400">
            Drivward primero recupera la conexión desde una USB, después completa lo que falta y, ya en el día a
            día, cuida que los drivers sigan estables.
          </p>
        </div>

        <ol className="grid grid-cols-1 gap-5">
          {STEPS.map((step, index) => (
            <li
              key={step.title}
              className="reveal group rounded-[2rem] border border-[#1f1f1f] bg-[#0e0e0e] p-7 transition-all duration-300 hover:-translate-y-1 hover:border-blue-500/30 hover:bg-[#111] md:p-8"
              style={{ transitionDelay: `${index * 60}ms` }}
            >
              <div className="mb-5 flex items-center justify-between">
                <span className="font-mono text-sm font-bold text-blue-500">0{index + 1}.</span>
                <step.icon
                  size={22}
                  aria-hidden="true"
                  className="text-neutral-600 transition-colors group-hover:text-blue-400"
                />
              </div>
              <h3 className="mb-3 text-2xl font-bold text-white">{step.title}</h3>
              <p className="text-neutral-500">{step.text}</p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
