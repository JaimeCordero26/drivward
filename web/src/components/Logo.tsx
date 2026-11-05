export default function Logo({ size = "md" }: { size?: "md" | "lg" }) {
  const box = size === "lg" ? "h-12 w-12 text-2xl rounded-xl" : "h-8 w-8 text-lg rounded-lg";
  return (
    <span
      aria-hidden="true"
      className={`${box} inline-flex items-center justify-center bg-blue-600 font-black tracking-tighter text-white transition-transform duration-300 group-hover:-rotate-6`}
    >
      D.
    </span>
  );
}
