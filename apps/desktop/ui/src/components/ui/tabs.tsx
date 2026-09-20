/* shadcn/ui Tabs — vendored on Radix primitives and themed to the Lumi
   tab strip: bottom hairline, active tab in Lumi Blue with the lit
   edge, keyboard roving focus and arrow-key navigation for free. */
import * as React from "react";
import * as TabsPrimitive from "@radix-ui/react-tabs";

import { cn } from "@/lib/utils";

function Tabs({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Root>) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      className={cn("flex flex-col", className)}
      {...props}
    />
  );
}

function TabsList({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.List>) {
  return (
    <TabsPrimitive.List
      data-slot="tabs-list"
      className={cn(
        "flex w-fit items-center gap-0.5 border-b border-[var(--color-border)] overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
        className,
      )}
      {...props}
    />
  );
}

function TabsTrigger({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      data-slot="tabs-trigger"
      className={cn(
        "inline-flex flex-none items-center justify-center gap-1.5 whitespace-nowrap rounded-t-[var(--radius-control)] border-b-2 border-transparent px-3 py-2 text-[13px] font-semibold text-muted-foreground outline-none transition-[color,background-color,border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-out)]",
        "hover:text-[var(--color-civic-navy)] hover:bg-secondary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-55",
        "data-[state=active]:text-primary data-[state=active]:border-b-primary data-[state=active]:shadow-[var(--shadow-lit)]",
        "[&_svg]:size-3.5 [&_svg]:pointer-events-none",
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return (
    <TabsPrimitive.Content
      data-slot="tabs-content"
      className={cn("mt-4 outline-none", className)}
      {...props}
    />
  );
}

export { Tabs, TabsList, TabsTrigger, TabsContent };
