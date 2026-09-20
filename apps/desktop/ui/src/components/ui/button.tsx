/* shadcn/ui Button — vendored and themed to DESIGN.md §10.1.
   Variants map 1:1 to the Lumi button hierarchy: `default` is the lit
   primary (gradient sheen + shadow-button), `secondary`/`outline` the
   calm bordered actions, `link` the tertiary inline action,
   `destructive` the rare red outline. Sizes follow the §10.1 contract
   (sm 32 / md 40 / lg 48 / icon-sm 32² / icon 40²). */
import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-[var(--radius-control)] text-sm font-[550] outline-none transition-[background-color,background-image,border-color,box-shadow,transform] duration-[var(--motion-fast)] ease-[var(--ease-out)] disabled:pointer-events-none disabled:opacity-55 [&_svg]:pointer-events-none [&_svg]:shrink-0 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-paper-white)]",
  {
    variants: {
      variant: {
        default:
          "border bg-primary bg-[image:var(--gradient-primary)] text-primary-foreground shadow-[var(--shadow-button)] border-[color-mix(in_oklch,var(--color-lumi-blue-active)_35%,transparent)] hover:bg-[image:none] hover:bg-primary-hover active:bg-primary-active active:shadow-none active:translate-y-px disabled:bg-primary disabled:shadow-none",
        secondary:
          "border border-[var(--color-border)] bg-transparent text-[var(--color-civic-navy)] shadow-[var(--shadow-lit)] hover:bg-secondary",
        outline:
          "border border-[var(--color-border)] bg-transparent text-[var(--color-civic-navy)] shadow-[var(--shadow-lit)] hover:bg-secondary",
        ghost:
          "border border-transparent bg-transparent text-[var(--color-civic-navy)] hover:bg-secondary",
        link: "text-primary font-[550] h-8 px-1 hover:underline underline-offset-4",
        destructive:
          "border border-[var(--color-risk-red)] bg-transparent text-[var(--color-risk-red)] shadow-[var(--shadow-lit)] hover:bg-[var(--color-red-soft)]",
      },
      size: {
        default: "h-10 px-4 min-w-24",
        sm: "h-8 px-3 text-[13px]",
        lg: "h-12 px-5 text-[15px] font-semibold",
        "icon-sm": "size-8 [&_svg]:size-4",
        icon: "size-10 [&_svg]:size-[18px]",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
