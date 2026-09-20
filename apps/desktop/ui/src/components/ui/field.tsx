import * as React from "react";

import { cn } from "@/lib/utils";

interface FieldProps extends React.ComponentProps<"div"> {
  id: string;
  label: string;
  children: React.ReactNode;
}

/** Lumi field wrapper: every application input gets a persistent label. */
function Field({ id, label, className, children, ...props }: FieldProps) {
  return (
    <div className={cn("field", className)} {...props}>
      <label className="field-label" htmlFor={id}>{label}</label>
      {children}
    </div>
  );
}

export { Field };
