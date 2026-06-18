import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import {
  ChartBarIcon,
  DocumentArrowUpIcon,
  PencilSquareIcon,
  PlayIcon,
} from "@heroicons/react/24/outline";
import Link from "next/link";

const actions = [
  { label: "New Evaluation", icon: PlayIcon, href: "/evaluate" },
  { label: "Upload Dataset", icon: DocumentArrowUpIcon, href: "/datasets" },
  { label: "Create Prompt", icon: PencilSquareIcon, href: "/prompts/new" },
  { label: "View Reports", icon: ChartBarIcon, href: "/results" },
];

export function QuickActions() {
  return (
    <Card>
      <h3 className="mb-4 text-sm font-semibold text-slate-900">Quick Actions</h3>
      <div className="grid gap-3 sm:grid-cols-2">
        {actions.map((action) => (

          <Button key={action.label} variant="secondary" className="justify-start gap-2">
            <Link href={action.href}>
              <action.icon className="h-4 w-4" />
              {action.label}
            </Link>
          </Button>
        ))}
      </div>
    </Card>
  );
}
