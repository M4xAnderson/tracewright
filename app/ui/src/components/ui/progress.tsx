// shadcn/ui Progress on Base UI. `value={null}` is indeterminate (flows §3.3, loading screen).
import { Progress as BaseProgress } from "@base-ui/react/progress";
import type { ComponentProps } from "react";
import { cn } from "../../lib/cn";

export function Progress({ className, ...props }: ComponentProps<typeof BaseProgress.Root>) {
  return (
    <BaseProgress.Root className={cn("w-full", className)} {...props}>
      <BaseProgress.Track className="h-1 w-full overflow-hidden rounded-full bg-surface-sunken">
        <BaseProgress.Indicator className="block h-full bg-accent data-[indeterminate]:w-1/3 data-[indeterminate]:animate-pulse" />
      </BaseProgress.Track>
    </BaseProgress.Root>
  );
}
