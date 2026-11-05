import { Globe } from "lucide-react";
import Github from "./GithubIcon";
import { CONTRIBUTING, DOCS, PRIVACY, REPO, SACORTECH } from "../links";

export default function Footer() {
  return (
    <footer className="relative border-t border-[#1a1a1a] px-4 pb-16 pt-32 sm:px-6 md:px-12 md:pt-48">
      <div className="mx-auto flex max-w-7xl flex-col items-start justify-between gap-16 md:flex-row md:items-end">
        <div className="reveal flex-1">
          <h2 className="mb-12 select-none text-[16vw] font-black uppercase leading-[0.85] tracking-tighter text-white md:text-[10vw]">
            Compu
            <br />
            lista<span className="text-blue-600">.</span>
          </h2>
          <div className="flex flex-col gap-6">
            <p className="text-2xl text-neutral-400">Una compu lista. Sin ir driver por driver.</p>
            <a
              href={REPO}
              className="inline-flex w-fit items-center gap-3 rounded-xl bg-white px-8 py-4 font-bold text-black transition-all hover:bg-blue-600 hover:text-white"
            >
              Seguir el proyecto
              <Github size={18} aria-hidden="true" />
            </a>
          </div>
        </div>

        <div className="flex gap-4 md:mb-6">
          <a
            href={REPO}
            aria-label="Drivward en GitHub"
            className="flex h-14 w-14 items-center justify-center rounded-full border border-[#333] transition-all hover:-translate-y-2 hover:bg-white hover:text-black"
          >
            <Github size={20} />
          </a>
          <a
            href={SACORTECH}
            aria-label="Sacor Tech"
            className="flex h-14 w-14 items-center justify-center rounded-full border border-[#333] transition-all hover:-translate-y-2 hover:bg-white hover:text-black"
          >
            <Globe size={20} />
          </a>
        </div>
      </div>

      <div className="mx-auto mt-32 flex max-w-7xl flex-col justify-between gap-6 border-t border-[#141414] pt-10 text-[11px] font-bold uppercase tracking-widest text-neutral-600 md:flex-row">
        <p>© 2026 Drivward · Sacor Tech. Código abierto bajo licencia GPL-3.0.</p>
        <div className="flex flex-wrap gap-8">
          <a href={DOCS} className="transition-colors hover:text-neutral-300">Documentación</a>
          <a href={CONTRIBUTING} className="transition-colors hover:text-neutral-300">Contribuir</a>
          <a href={PRIVACY} className="transition-colors hover:text-neutral-300">Privacidad</a>
        </div>
      </div>
      <p className="mx-auto mt-6 max-w-7xl text-xs text-neutral-700">
        Windows es una marca de Microsoft. Drivward es un proyecto independiente y no está afiliado a fabricantes de
        hardware ni a Microsoft.
      </p>
    </footer>
  );
}
