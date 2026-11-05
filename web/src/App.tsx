import Nav from "./components/Nav";
import Hero from "./components/Hero";
import HowItWorks from "./components/HowItWorks";
import Products from "./components/Products";
import Benefits from "./components/Benefits";
import Technicians from "./components/Technicians";
import Footer from "./components/Footer";
import { useReveal } from "./useReveal";

export default function App() {
  useReveal();
  return (
    <div className="min-h-screen overflow-x-hidden bg-ink">
      <a
        href="#contenido"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[200] focus:rounded-lg focus:bg-white focus:px-4 focus:py-2 focus:text-black"
      >
        Saltar al contenido
      </a>
      <Nav />
      <Hero />
      <main id="contenido">
        <HowItWorks />
        <Products />
        <Benefits />
        <Technicians />
      </main>
      <Footer />
    </div>
  );
}
