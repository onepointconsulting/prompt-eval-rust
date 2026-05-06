type PageContainerProps = {
  title: string;
  description?: string;
  actions?: React.ReactNode;
  children: React.ReactNode;
};

export function PageContainer({
  title,
  description,
  actions,
  children,
}: PageContainerProps) {
  return (
    <section className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-slate-900">{title}</h1>
          {description && <p className="mt-1 text-sm text-slate-600">{description}</p>}
        </div>
        {actions}
      </div>
      {children}
    </section>
  );
}
