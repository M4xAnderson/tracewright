import type { ComponentProps } from "react";
import { cn } from "../../lib/cn";

export function Kbd({ className, ...props }: ComponentProps<"kbd">) {
  return (
    <kbd
      className={cn(
        "rounded-sm border border-border-hairline px-1 font-sans text-[11px] text-current opacity-80",
        className,
      )}
      {...props}
    />
  );
}
