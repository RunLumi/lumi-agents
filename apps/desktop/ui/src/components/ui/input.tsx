/* shadcn/ui Input + Textarea — vendored and themed to DESIGN.md §10.3:
   calm white fields, recessed shadow-input, blue border + 3px halo on
   focus. Never a label substitute for the placeholder. */
import * as React from "react";

import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "flex h-10 w-full min-w-0 rounded-[var(--radius-control)] border border-[var(--color-border)] bg-card px-3 text-base text-[var(--color-ink)] shadow-[var(--shadow-input)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] outline-none",
        "placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground",
        "hover:border-[var(--color-border-strong)]",
        "focus-visible:border-primary focus-visible:ring-[3px] focus-visible:ring-[color-mix(in_oklch,var(--color-lumi-blue)_14%,transparent)]",
        "disabled:border-[var(--color-border-subtle)] disabled:bg-transparent disabled:text-muted-foreground disabled:shadow-none",
        "aria-invalid:border-[var(--color-risk-red)] aria-invalid:ring-[color-mix(in_oklch,var(--color-risk-red)_14%,transparent)]",
        className,
      )}
      {...props}
    />
  );
}

function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "flex min-h-20 w-full rounded-[var(--radius-control)] border border-[var(--color-border)] bg-card px-3 py-2.5 text-base text-[var(--color-ink)] shadow-[var(--shadow-input)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] outline-none",
        "placeholder:text-muted-foreground",
        "hover:border-[var(--color-border-strong)]",
        "focus-visible:border-primary focus-visible:ring-[3px] focus-visible:ring-[color-mix(in_oklch,var(--color-lumi-blue)_14%,transparent)]",
        "disabled:border-[var(--color-border-subtle)] disabled:bg-transparent disabled:text-muted-foreground disabled:shadow-none",
        "aria-invalid:border-[var(--color-risk-red)] aria-invalid:ring-[color-mix(in_oklch,var(--color-risk-red)_14%,transparent)]",
        className,
      )}
      {...props}
    />
  );
}

export { Input, Textarea };
