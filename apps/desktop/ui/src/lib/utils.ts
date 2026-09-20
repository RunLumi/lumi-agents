import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** shadcn class combiner: clsx for conditionals, tailwind-merge so later
    utilities override earlier ones instead of fighting the cascade. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
