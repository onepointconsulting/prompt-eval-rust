"use client";

import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { ArrowUpTrayIcon } from "@heroicons/react/24/outline";
import { useRouter } from "next/navigation";

export function DatasetUploader() {
  const router = useRouter();

  return (
    <Card className="border-dashed">
      <div className="flex flex-col items-center gap-3 py-6 text-center">
        <ArrowUpTrayIcon className="h-8 w-8 text-slate-400" />
        <div>
          <p className="font-medium text-slate-800">Upload New Dataset</p>
          <p className="text-sm text-slate-500">JSON format • Max 10MB</p>
        </div>
        <Button variant="secondary" onClick={() => router.push("/datasets/upload")}>
          Click to upload
        </Button>
      </div>
    </Card>
  );
}
