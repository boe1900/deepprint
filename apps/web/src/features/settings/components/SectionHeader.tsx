export function SectionHeader({
  description,
  title,
}: {
  description: string;
  title: string;
}) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-xl border bg-muted/20 px-4 py-3">
      <div className="min-w-0">
        <h4 className="font-heading truncate text-sm font-medium text-foreground">{title}</h4>
        <p className="mt-0.5 truncate text-xs text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}
