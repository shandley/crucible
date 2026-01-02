import { cn } from '../lib/utils'

interface BadgeProps {
  children: React.ReactNode
  variant?: 'default' | 'secondary' | 'success' | 'destructive' | 'warning'
  className?: string
}

export function Badge({ children, variant = 'default', className }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium',
        {
          'bg-foreground text-background': variant === 'default',
          'bg-muted text-muted-foreground': variant === 'secondary',
          'bg-success/10 text-success': variant === 'success',
          'bg-destructive/10 text-destructive': variant === 'destructive',
          'bg-warning/10 text-warning': variant === 'warning',
        },
        className
      )}
    >
      {children}
    </span>
  )
}
