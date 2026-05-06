import { cn } from "@/lib/utils";
import type { ButtonHTMLAttributes } from "react";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "secondary" | "ghost" | "danger";
  size?: "default" | "sm" | "icon";
};

export function Button({
  className,
  variant = "default",
  size = "default",
  ...props
}: ButtonProps) {
  return (
    <button
      className={cn(
        "inline-flex cursor-pointer items-center justify-center rounded-lg font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-60",
        variant === "default" && "bg-blue-600 text-white hover:bg-blue-700",
        variant === "secondary" &&
        "border border-slate-300 bg-white text-slate-700 hover:bg-slate-50",
        variant === "ghost" && "text-slate-700 hover:bg-slate-100",
        variant === "danger" && "bg-red-600 text-white hover:bg-red-700",
        size === "default" && "h-10 px-4 text-sm",
        size === "sm" && "h-8 px-3 text-xs",
        size === "icon" && "h-9 w-9",
        className
      )}
      {...props}
    />
  );
}
