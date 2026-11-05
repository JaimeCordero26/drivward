import { useEffect, useState } from "react";
import { Menu, X } from "lucide-react";
import Github from "./GithubIcon";
import Logo from "./Logo";
import { RELEASES, REPO } from "../links";

const LINKS = [
  { href: "#como-funciona", label: "Cómo funciona" },
  { href: "#versiones", label: "Versiones" },
  { href: "#principios", label: "Principios" },
  { href: "#tecnicos", label: "Para técnicos" },
];

export default function Nav() {
  const [scrolled, setScrolled] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 24);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav
      aria-label="Principal"
      className={`fixed inset-x-0 top-0 z-[100] transition-all duration-300 ${
        scrolled || open ? "border-b border-white/5 bg-ink/80 backdrop-blur-xl" : "bg-transparent"
      }`}
    >
      <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-4 text-sm font-medium sm:px-6 md:py-5">
        <div className="flex items-center gap-10">
          <a href="#" className="group flex items-center gap-3" aria-label="Drivward, inicio">
            <Logo />
            <span className="text-base font-bold tracking-tight text-white">Drivward</span>
          </a>
          <div className="hidden items-center gap-8 text-neutral-400 lg:flex">
            {LINKS.map((link) => (
              <a key={link.href} href={link.href} className="transition-colors hover:text-white">
                {link.label}
              </a>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-3">
          <a
            href={REPO}
            className="hidden items-center gap-2 rounded-full border border-white/10 px-4 py-2 text-neutral-400 transition-colors hover:border-white/25 hover:text-white md:flex"
          >
            <Github size={16} aria-hidden="true" />
            GitHub
          </a>
          <a
            href={RELEASES}
            className="rounded-lg bg-white px-4 py-2 font-bold text-black transition-colors duration-300 hover:bg-blue-600 hover:text-white sm:px-5 sm:py-2.5"
          >
            Descargar
          </a>
          <button
            type="button"
            className="rounded-lg p-2 text-neutral-300 hover:text-white lg:hidden"
            aria-label={open ? "Cerrar menú" : "Abrir menú"}
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            {open ? <X size={20} /> : <Menu size={20} />}
          </button>
        </div>
      </div>

      {open && (
        <div className="border-t border-white/5 px-4 pb-6 pt-2 sm:px-6 lg:hidden">
          {LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              onClick={() => setOpen(false)}
              className="block py-3 text-lg font-medium text-neutral-300 hover:text-white"
            >
              {link.label}
            </a>
          ))}
          <a href={REPO} className="mt-2 flex items-center gap-2 py-3 text-lg text-neutral-300">
            <Github size={18} aria-hidden="true" /> GitHub
          </a>
        </div>
      )}
    </nav>
  );
}
