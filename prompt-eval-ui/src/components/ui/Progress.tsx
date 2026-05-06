type ProgressProps = {
  value: number;
};

export function Progress({ value }: ProgressProps) {
  const clamped = Math.max(0, Math.min(100, value));
  return (
    <div className="h-2 w-full rounded-full bg-slate-200">
      <div
        className="h-2 rounded-full bg-blue-600 transition-all"
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}
