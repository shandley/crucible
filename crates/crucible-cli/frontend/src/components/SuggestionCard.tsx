import { useState } from 'react'
import type { SuggestionInfo, DecisionInfo } from '../types'
import { Card, CardHeader, CardContent, CardFooter } from './Card'
import { Badge } from './Badge'
import { Button } from './Button'
import { cn } from '../lib/utils'

interface SuggestionCardProps {
  suggestion: SuggestionInfo
  decision?: DecisionInfo
  onAccept: (notes?: string) => void
  onReject: (notes: string) => void
}

const actionColors: Record<string, 'default' | 'warning' | 'destructive' | 'success'> = {
  Standardize: 'default',
  Flag: 'warning',
  Derive: 'success',
  Split: 'default',
  Merge: 'default',
  Drop: 'destructive',
  Rename: 'default',
  Retype: 'default',
}

export function SuggestionCard({
  suggestion,
  decision,
  onAccept,
  onReject,
}: SuggestionCardProps) {
  const [notes, setNotes] = useState('')
  const [showNotes, setShowNotes] = useState(false)

  const isPending = !decision || decision.status === 'pending'
  const isAccepted = decision?.status === 'accepted'
  const isRejected = decision?.status === 'rejected'

  return (
    <Card
      className={cn({
        'border-success/50 bg-success/5': isAccepted,
        'border-destructive/50 bg-destructive/5': isRejected,
      })}
    >
      <CardHeader>
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2">
            <Badge variant={actionColors[suggestion.action] || 'default'}>
              {suggestion.action}
            </Badge>
            <span className="text-sm text-muted-foreground">
              P{suggestion.priority}
            </span>
          </div>
          <span className="text-sm font-medium">
            {Math.round(suggestion.confidence * 100)}%
          </span>
        </div>
      </CardHeader>

      <CardContent>
        <p className="text-sm text-muted-foreground">
          {suggestion.rationale}
        </p>
        {suggestion.affected_rows > 0 && (
          <p className="mt-2 text-xs text-muted-foreground">
            Affects {suggestion.affected_rows} row(s)
          </p>
        )}
        {decision?.notes && (
          <p className="mt-2 rounded bg-muted p-2 text-xs">
            Note: {decision.notes}
          </p>
        )}
      </CardContent>

      {isPending && (
        <CardFooter className="flex-col gap-2">
          {showNotes && (
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Add notes (required for reject)"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2"
              rows={2}
            />
          )}
          <div className="flex w-full gap-2">
            <Button
              variant="success"
              size="sm"
              className="flex-1"
              onClick={() => onAccept(notes || undefined)}
            >
              Accept
            </Button>
            <Button
              variant="destructive"
              size="sm"
              className="flex-1"
              onClick={() => {
                if (!notes) {
                  setShowNotes(true)
                  return
                }
                onReject(notes)
              }}
            >
              Reject
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowNotes(!showNotes)}
            >
              {showNotes ? '−' : '+'}
            </Button>
          </div>
        </CardFooter>
      )}

      {!isPending && (
        <CardFooter>
          <Badge variant={isAccepted ? 'success' : 'destructive'}>
            {decision?.status}
          </Badge>
        </CardFooter>
      )}
    </Card>
  )
}
