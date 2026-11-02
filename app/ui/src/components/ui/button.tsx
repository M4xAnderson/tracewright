// shadcn/ui Button on Base UI, restyled to the option 2b tokens through variant and size (R-5).
import { Button as BaseButton } from "@base-ui/react/button";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "../../lib/cn";

const buttonVariants = cva(
  "inline-flex shrink-0 cursor-pointer items-center justify-center gap-1.5 rounded border font-sans whitespace-nowrap disabled:cursor-default disabled:opacity-60 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "border-border-control bg-surface-raised text-text-primary hover:bg-surface-hover",
        primary: "border-accent bg-accent font-medium text-on-accent hover:bg-accent-hover",
        ghost:
          "border-transparent bg-transparent text-text-secondary hover:bg-surface-hover hover:text-text-primary",
      },
      size: {
        default: "h-7 px-3",
        large: "h-9 px-4 text-sm",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

export function Button({
  className,
  variant,
  size,
  ...props
}: ComponentProps<typeof BaseButton> & VariantProps<typeof buttonVariants>) {
  return <BaseButton className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}
