import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";

export function ExportOptions() {
  return (
    <Card className="flex flex-wrap gap-2">
      <Button variant="secondary">Export PDF</Button>
      <Button variant="secondary">Export CSV</Button>
      <Button>Share</Button>
      <Button variant="secondary">Re-run</Button>
    </Card>
  );
}
