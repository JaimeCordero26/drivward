import { Check } from "lucide-react";
import { REQUIREMENTS } from "../links";

const FEATURES = [
  { title: "Librería verificable", text: "Carpeta de drivers con index.json y hash SHA-256 de cada archivo, comprobado antes de instalar." },
  { title: "Kits por modelo", text: "Guarda los drivers que funcionaron en un modelo y reutilízalos en el siguiente equipo igual." },
  { title: "Reportes de servicio", text: "Exporta el análisis y lo que se hizo en HTML y JSON, sin datos personales innecesarios." },
  { title: "Registro de cada acción", text: "Fecha, dispositivo, versión anterior, versión nueva, salida de pnputil y resultado." },
  { title: "Explica cada elección", text: "Coincidencia de Hardware ID, firma, origen, versión y fecha: ves por qué ganó cada driver." },
  { title: "Preparador de USB", text: "Descarga los paquetes oficiales y arma la USB en tu compu, registrando origen, licencia y hash." },
];

const STACK = ["Rust", "SetupAPI · CfgMgr32", "WMI", "Windows Update Agent", "pnputil · DISM", "SQLite", "Tauri + React"];

export default function Technicians() {
  return (
    <section id="tecnicos" className="scroll-mt-20 px-4 py-24 sm:px-6 md:px-12 md:py-32">
      <div className="reveal mx-auto max-w-7xl rounded-[2.5rem] border border-white/5 bg-[#0e0e0e] p-8 md:p-16 lg:p-20">
        <div className="max-w-3xl">
          <h2 className="mb-6 text-4xl font-bold tracking-tighter text-white md:text-5xl">Para técnicos</h2>
          <p className="mb-12 text-xl text-neutral-400">
            Pensado primero para quien formatea y mantiene compus de clientes, y seguro también para quien no quiere
            decidir detalles técnicos.
          </p>
        </div>

        <ul className="grid grid-cols-1 gap-8 text-white md:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((feature) => (
            <li key={feature.title} className="flex items-start gap-4">
              <span className="mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded bg-blue-600">
                <Check size={12} className="text-white" aria-hidden="true" />
              </span>
              <div>
                <span className="block font-bold">{feature.title}</span>
                <span className="text-sm text-neutral-500">{feature.text}</span>
              </div>
            </li>
          ))}
        </ul>

        <div className="mt-14 flex flex-wrap gap-2 border-t border-white/5 pt-10">
          {STACK.map((item) => (
            <span
              key={item}
              className="rounded-full border border-white/10 px-3 py-1 font-mono text-xs text-neutral-400 transition-colors hover:border-blue-500/40 hover:text-white"
            >
              {item}
            </span>
          ))}
        </div>
        <p className="mt-6 text-sm text-neutral-500">
          Windows 10 y 11 x64. Linux y ARM64 quedan para fases posteriores.{" "}
          <a href={REQUIREMENTS} className="text-blue-400 underline-offset-4 hover:underline">
            Leer los requerimientos
          </a>
        </p>
      </div>
    </section>
  );
}
