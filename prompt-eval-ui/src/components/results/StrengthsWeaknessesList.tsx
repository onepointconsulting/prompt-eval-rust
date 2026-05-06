type StrengthsWeaknessesListProps = {
  strengths: string[];
  weaknesses: string[];
};

export function StrengthsWeaknessesList({
  strengths,
  weaknesses,
}: StrengthsWeaknessesListProps) {
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <div>
        <p className="mb-1 text-xs font-semibold text-green-700">Strengths</p>
        <ul className="list-disc space-y-1 pl-4 text-sm text-slate-600">
          {strengths.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </div>
      <div>
        <p className="mb-1 text-xs font-semibold text-orange-700">Weaknesses</p>
        <ul className="list-disc space-y-1 pl-4 text-sm text-slate-600">
          {weaknesses.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}
