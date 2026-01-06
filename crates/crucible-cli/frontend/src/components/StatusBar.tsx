import type { SummaryInfo } from '../types'
import { Progress } from './Progress'

interface StatusBarProps {
  summary: SummaryInfo
  filename: string
  progress: number
  llmStatus?: {
    available: boolean
    provider: string | null
  }
}

export function StatusBar({ summary, filename, progress, llmStatus }: StatusBarProps) {
  const reviewed = summary.total_suggestions - summary.pending_count

  return (
    <div className="border-b bg-background px-6 py-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div>
            <h1 className="text-lg font-semibold">Crucible</h1>
            <p className="text-sm text-muted-foreground">{filename}</p>
          </div>
          {llmStatus && (
            <div
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${
                llmStatus.available
                  ? 'bg-emerald-100 text-emerald-700'
                  : 'bg-gray-100 text-gray-500'
              }`}
              title={
                llmStatus.available
                  ? `AI powered by ${llmStatus.provider}`
                  : 'Set ANTHROPIC_API_KEY or OPENAI_API_KEY to enable AI features'
              }
            >
              <span
                className={`w-2 h-2 rounded-full ${
                  llmStatus.available ? 'bg-emerald-500' : 'bg-gray-400'
                }`}
              />
              {llmStatus.available ? `AI: ${llmStatus.provider}` : 'AI: Off'}
            </div>
          )}
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
