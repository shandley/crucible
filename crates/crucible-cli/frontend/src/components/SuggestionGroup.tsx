import { Button } from './Button'
import { Badge } from './Badge'

interface SuggestionGroupProps {
  column: string
  pendingCount: number
  totalCount: number
  isExpanded: boolean
  onToggle: () => void
  onAcceptAll: () => void
  onRejectAll: () => void
  isAccepting?: boolean
  isRejecting?: boolean
  children: React.ReactNode
}

export function SuggestionGroup({
  column,
  pendingCount,
  totalCount,
  isExpanded,
  onToggle,
  onAcceptAll,
  onRejectAll,
  isAccepting,
  isRejecting,
  children,
}: SuggestionGroupProps) {
  return (
    <div className="rounded-lg border bg-background">
      <div
        className="flex cursor-pointer items-center justify-between px-4 py-3 hover:bg-muted/50"
        onClick={onToggle}
      >
        <div className="flex items-center gap-3">
          <span className="text-muted-foreground">
            {isExpanded ? '▼' : '▶'}
          </span>
          <span className="font-medium">{column}</span>
          <Badge variant={pendingCount > 0 ? 'warning' : 'success'}>
            {pendingCount > 0 ? `${pendingCount} pending` : 'done'}
          </Badge>
          <span className="text-xs text-muted-foreground">
            {totalCount} suggestion{totalCount !== 1 ? 's' : ''}
          </span>
        </div>
        {pendingCount > 0 && (
          <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
            <Button
              variant="success"
              size="sm"
              onClick={onAcceptAll}
              disabled={isAccepting || isRejecting}
            >
              {isAccepting ? 'Accepting...' : 'Accept'}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={onRejectAll}
              disabled={isAccepting || isRejecting}
            >
              {isRejecting ? 'Rejecting...' : 'Reject'}
            </Button>
          </div>
        )}
      </div>
      {isExpanded && (
        <div className="border-t px-4 py-3 space-y-3">
          {children}
        </div>
      )}
    </div>
  )
}
