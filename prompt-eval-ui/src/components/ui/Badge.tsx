import { cn } from "@/lib/utils";
import type { HTMLAttributes } from "react";

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  variant?: "default" | "success" | "warning" | "danger" | "neutral";
};

export function Badge({ className, variant = "default", ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2.5 py-1 text-xs font-semibold",
        variant === "default" && "bg-blue-100 text-blue-700",
        variant === "success" && "bg-green-100 text-green-700",
        variant === "warning" && "bg-orange-100 text-orange-700",
        variant === "danger" && "bg-red-100 text-red-700",
        variant === "neutral" && "bg-slate-100 text-slate-700",
        className
      )}
      {...props}
    />
  );
}
