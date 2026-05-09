import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium transition-colors",
  {
    variants: {
      variant: {
        default: "border-transparent bg-white/8 text-foreground",
        secondary: "border-transparent bg-white/6 text-muted-foreground",
        success: "border-transparent bg-[color:var(--signal-success-bg)] text-[color:var(--signal-success-text)]",
        warning: "border-transparent bg-[color:var(--signal-warning-bg)] text-[color:var(--signal-warning-text)]",
        danger: "border-transparent bg-[color:var(--signal-danger-bg)] text-[color:var(--signal-danger-text)]",
        info: "border-transparent bg-[color:var(--signal-info-bg)] text-[color:var(--signal-info-text)]",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
