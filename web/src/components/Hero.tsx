import { ArrowDown, ShieldCheck } from "lucide-react";
import { RELEASES, REPO } from "../links";

export default function Hero() {
  return (
    <header className="relative flex min-h-[100svh] w-full flex-col items-center justify-center px-4 pb-24 pt-28 sm:px-6">
      <div className="grain pointer-events-none absolute inset-0">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_45%,#0f1a33_0%,#050505_65%)]" />
        <div className="absolute left-1/2 top-1/3 h-[40vh] w-[60vw] -translate-x-1/2 rounded-full bg-blue-600/10 blur-[120px]" />
      </div>

      <div className="relative z-10 flex flex-col items-center">
        <div className="animate-rise mb-8 inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-4 py-1.5 text-center text-[10px] font-bold uppercase tracking-[0.2em] text-neutral-400 sm:text-[11px]">
          <span className="h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-blue-500" />
          En desarrollo · Gratis · Código abierto · GPL-3.0
        </div>

        <h1 className="hero-title animate-rise text-center font-bold text-white [animation-delay:80ms]">
          Drivward
        </h1>

        <p className="animate-rise mt-8 max-w-2xl text-center text-xl font-medium leading-tight tracking-tight text-neutral-400 [animation-delay:160ms] md:text-3xl">
          Tus drivers, en orden. <br className="hidden md:block" />
          Desde el formateo hasta el día a día.
        </p>

        <div className="animate-rise mt-12 flex w-full flex-col items-center gap-4 [animation-delay:240ms] sm:w-auto sm:flex-row">
          <a
            href={RELEASES}
            className="w-full rounded-xl bg-blue-600 px-10 py-4 text-center text-lg font-bold text-white transition-all hover:-translate-y-0.5 hover:bg-blue-500 hover:shadow-[0_12px_40px_-12px_rgba(59,130,246,0.7)] sm:w-auto"
          >
            Descargar
          </a>
          <a
            href={REPO}
            className="w-full rounded-xl border border-white/10 bg-white/5 px-10 py-4 text-center text-lg font-bold text-white transition-all hover:border-white/20 hover:bg-white/10 sm:w-auto"
          >
            Ver en GitHub
          </a>
        </div>
        <p className="animate-rise mt-5 text-center text-sm text-neutral-500 [animation-delay:320ms]">
          Para Windows 10 y 11 de 64 bits. Todavía no hay una versión estable.
        </p>
      </div>

      <div className="absolute bottom-8 left-4 hidden items-center gap-4 sm:flex md:left-12">
        <div className="rounded-full border border-blue-500/20 bg-blue-600/10 p-3">
          <ShieldCheck size={20} className="text-blue-400" aria-hidden="true" />
        </div>
        <p className="text-xs font-bold uppercase tracking-widest text-neutral-500">
          Nada se instala sin tu aprobación
        </p>
      </div>
      <a
        href="#como-funciona"
        aria-label="Ir a cómo funciona"
        className="absolute bottom-8 right-4 rounded-full border border-white/10 p-3 text-neutral-500 transition-colors hover:text-white md:right-12"
      >
        <ArrowDown size={18} className="animate-bounce" />
      </a>
    </header>
  );
}
