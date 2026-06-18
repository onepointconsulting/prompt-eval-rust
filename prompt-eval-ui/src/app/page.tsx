"use client";

import { PerformanceChart } from "@/components/dashboard/PerformanceChart";
import { QuickActions } from "@/components/dashboard/QuickActions";
import { RecentEvalsList } from "@/components/dashboard/RecentEvalsList";
import { StatCard } from "@/components/dashboard/StatCard";
import { TopPromptsList } from "@/components/dashboard/TopPromptsList";
import { PageContainer } from "@/components/layout/PageContainer";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import type { DashboardStats, EvalSummary, PromptTemplate, TrendPoint } from "@/lib/types";
import {
  ArrowPathIcon,
  ChartBarIcon,
  CheckCircleIcon,
  DocumentTextIcon,
  RocketLaunchIcon,
} from "@heroicons/react/24/outline";
import { useSession } from "next-auth/react";

const emptyStats: DashboardStats = {
  totalEvals: 0,
  activePrompts: 0,
  avgScore: 0,
  successRate: 0,
};
const emptyTrend: TrendPoint[] = [];
const emptyEvals: EvalSummary[] = [];
const emptyPrompts: PromptTemplate[] = [];

export default function Home() {
  const stats = useApiData(api.getDashboardStats, emptyStats, "Dashboard stats");
  const trend = useApiData(api.getPerformanceTrend, emptyTrend, "Performance trend");
  const recent = useApiData(api.getRecentEvals, emptyEvals, "Recent evaluations");
  const topPrompts = useApiData(api.getTopPrompts, emptyPrompts, "Top prompts");
  const session = useSession();
  console.log("session: ", session);
  return (
    <PageContainer
      title="Dashboard"
      description="Track prompt performance, evaluation health, and recent activity."
      actions={
        <Button>
          <ArrowPathIcon className="mr-2 h-4 w-4" />
          Refresh
        </Button>
      }
    >
      {stats.loading && <Card className="text-sm text-slate-500">Loading dashboard data...</Card>}
      {stats.error && (
        <Card className="border-orange-200 bg-orange-50 text-sm text-orange-700">
          {stats.error}
        </Card>
      )}
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          title="Total Evaluations"
          value={`${stats.data?.totalEvals ?? 0}`}
          icon={ChartBarIcon}
        />
        <StatCard
          title="Active Prompts"
          value={`${stats.data?.activePrompts ?? 0}`}
          icon={DocumentTextIcon}
        />
        <StatCard
          title="Average Score"
          value={`${(stats.data?.avgScore ?? 0).toFixed(1)} / 10`}
          icon={CheckCircleIcon}
        />
        <StatCard
          title="Success Rate"
          value={`${(stats.data?.successRate ?? 0).toFixed(1)}%`}
          hint="Evals with avg score ≥ 7.0"
          icon={RocketLaunchIcon}
        />
      </div>
      <PerformanceChart data={trend.data ?? emptyTrend} />
      <div className="grid gap-4 lg:grid-cols-2">
        <RecentEvalsList items={recent.data ?? emptyEvals} />
        <TopPromptsList prompts={topPrompts.data ?? emptyPrompts} />
      </div>
      <QuickActions />
    </PageContainer>
  );
}
