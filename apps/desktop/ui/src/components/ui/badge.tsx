/* shadcn/ui Badge — vendored and themed to DESIGN.md §10.4, the
   "lit token" system. Status variants map to the only allowed families
   (open/in-progress/done/overdue/critical); counts and tags use the
   dotless secondary/outline so tags never pose as status. Each family
   is a quad of §10.4 tokens (surface/ring/dot/text) defined in
   index.css; `dot` renders the 5px saturated disc, `interactive` earns
   the hover-lift/focus states. */
import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex w-fit shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-full min-h-5 px-2 text-xs font-[560] tracking-[0.01em] tabular-nums shadow-[var(--shadow-lit)] ring-1 ring-inset",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground",
        secondary:
          "bg-card text-secondary-foreground ring-[var(--color-border)]",
        outline: "bg-card text-secondary-foreground ring-[var(--color-border)]",
        "status-open":
          "bg-[var(--badge-neutral-surface)] text-[var(--badge-neutral-text)] ring-[var(--badge-neutral-ring)] [--badge-dot:var(--badge-neutral-dot)]",
        "status-in-progress":
          "bg-[var(--badge-info-surface)] text-[var(--badge-info-text)] ring-[var(--badge-info-ring)] [--badge-dot:var(--badge-info-dot)]",
        "status-done":
          "bg-[var(--badge-success-surface)] text-[var(--badge-success-text)] ring-[var(--badge-success-ring)] [--badge-dot:var(--badge-success-dot)]",
        "status-overdue":
          "bg-[var(--badge-warning-surface)] text-[var(--badge-warning-text)] ring-[var(--badge-warning-ring)] [--badge-dot:var(--badge-warning-dot)]",
        "status-critical":
          "bg-[var(--badge-danger-surface)] text-[var(--badge-danger-text)] ring-[var(--badge-danger-ring)] [--badge-dot:var(--badge-danger-dot)]",
      },
      interactive: {
        true: "cursor-pointer transition-[filter,translate] duration-[var(--motion-fast)] ease-[var(--ease-out)] hover:brightness-[0.985] hover:-translate-y-px active:translate-y-0 active:brightness-[0.95] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        false: "",
      },
    },
    defaultVariants: {
      variant: "default",
      interactive: false,
    },
  },
);

function Badge({
  className,
  variant,
  interactive,
  dot = false,
  asChild = false,
  ...props
}: React.ComponentProps<"span"> &
  VariantProps<typeof badgeVariants> & {
    /** Render the 5px saturated status disc (§10.4). Implied for status variants via `withDot`. */
    dot?: boolean;
    asChild?: boolean;
  }) {
  const Comp = asChild ? Slot : "span";
  return (
    <Comp
      data-slot="badge"
      className={cn(badgeVariants({ variant, interactive, className }))}
      {...props}
    >
      {dot && (
        <span
          aria-hidden
          data-slot="badge-dot"
          className="size-[5px] rounded-full bg-[var(--badge-dot,var(--color-soft-slate))]"
        />
      )}
      {props.children}
    </Comp>
  );
}

/** Status badges carry their dot automatically (§10.4: status only). */
function StatusBadge({
  variant,
  children,
  ...props
}: Omit<React.ComponentProps<typeof Badge>, "dot" | "variant"> &
  Required<Pick<VariantProps<typeof badgeVariants>, "variant">>) {
  return (
    <Badge variant={variant} dot {...props}>
      {children}
    </Badge>
  );
}

export { Badge, StatusBadge, badgeVariants };

/** The §10.4 variant families, for callers that map domain status to a
    variant (e.g. task status → status-done). */
export type BadgeVariant = NonNullable<VariantProps<typeof badgeVariants>["variant"]>;
