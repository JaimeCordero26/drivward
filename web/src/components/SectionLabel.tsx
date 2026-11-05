export default function SectionLabel({ children }: { children: string }) {
  return (
    <div className="mb-8 flex items-center gap-3">
      <span className="h-2 w-2 rounded-full bg-blue-500" />
      <span className="text-[11px] font-bold uppercase tracking-[0.3em] text-neutral-500">{children}</span>
    </div>
  );
}
