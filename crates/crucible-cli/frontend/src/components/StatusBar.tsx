import type { SummaryInfo } from '../types'
import { Progress } from './Progress'

interface StatusBarProps {
  summary: SummaryInfo
  filename: string
  progress: number
}

export function StatusBar({ summary, filename, progress }: StatusBarProps) {
  const reviewed = summary.total_suggestions - summary.pending_count

  return (
    <div className="border-b bg-background px-6 py-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold">Crucible</h1>
          <p className="text-sm text-muted-foreground">{filename}</p>
        </div>
        <div className="text-right">
          <div className="text-sm font-medium">
            {reviewed} / {summary.total_suggestions} reviewed
          </div>
          <div className="text-xs text-muted-foreground">
            {summary.accepted_count} accepted · {summary.rejected_count} rejected
          </div>
        </div>
      </div>
      <div className="mt-3">
        <Progress value={progress} />
      </div>
    </div>
  )
}
