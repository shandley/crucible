import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getCuration, acceptDecision, rejectDecision, saveCuration } from './api/client'
import { SuggestionCard, StatusBar, Button } from './components'
import type { DecisionInfo, SuggestionInfo } from './types'

function findDecision(decisions: DecisionInfo[], suggestionId: string): DecisionInfo | undefined {
  return decisions.find(d => d.suggestion_id === suggestionId)
}

function isPending(decisions: DecisionInfo[], suggestion: SuggestionInfo): boolean {
  const decision = findDecision(decisions, suggestion.id)
  return !decision || decision.status === 'pending'
}

export default function App() {
  const queryClient = useQueryClient()

  const { data: curation, isLoading, error } = useQuery({
    queryKey: ['curation'],
    queryFn: getCuration,
  })

  const acceptMutation = useMutation({
    mutationFn: ({ id, notes }: { id: string; notes?: string }) =>
      acceptDecision(id, notes),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
    },
  })

  const rejectMutation = useMutation({
    mutationFn: ({ id, notes }: { id: string; notes: string }) =>
      rejectDecision(id, notes),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
    },
  })

  const saveMutation = useMutation({
    mutationFn: saveCuration,
  })

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <p className="text-muted-foreground">Loading...</p>
      </div>
    )
  }

  if (error || !curation) {
    return (
      <div className="flex h-screen items-center justify-center">
        <p className="text-destructive">
          Error loading curation: {error?.message || 'Unknown error'}
        </p>
      </div>
    )
  }

  const pendingSuggestions = curation.suggestions.filter(s =>
    isPending(curation.decisions, s)
  )
  const reviewedSuggestions = curation.suggestions.filter(s =>
    !isPending(curation.decisions, s)
  )

  // Sort pending by priority (lower = higher priority)
  pendingSuggestions.sort((a, b) => a.priority - b.priority)

  return (
    <div className="min-h-screen bg-muted/30">
      <StatusBar
        summary={curation.summary}
        filename={curation.source.file}
        progress={curation.progress}
      />

      <div className="mx-auto max-w-4xl px-6 py-6">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-medium">
            {curation.summary.pending_count > 0
              ? `${curation.summary.pending_count} pending review`
              : 'All suggestions reviewed'}
          </h2>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => saveMutation.mutate()}
            disabled={saveMutation.isPending}
          >
            {saveMutation.isPending ? 'Saving...' : 'Save'}
          </Button>
        </div>

        {curation.summary.pending_count === 0 && (
          <div className="mb-6 rounded-lg border border-success/50 bg-success/5 p-4 text-center">
            <p className="font-medium text-success">Review complete!</p>
            <p className="mt-1 text-sm text-muted-foreground">
              Run <code className="rounded bg-muted px-1">crucible apply</code> to
              generate your curated dataset.
            </p>
          </div>
        )}

        <div className="space-y-4">
          {pendingSuggestions.map((suggestion) => (
            <SuggestionCard
              key={suggestion.id}
              suggestion={suggestion}
              decision={findDecision(curation.decisions, suggestion.id)}
              onAccept={(notes) =>
                acceptMutation.mutate({ id: suggestion.id, notes })
              }
              onReject={(notes) =>
                rejectMutation.mutate({ id: suggestion.id, notes })
              }
            />
          ))}
        </div>

        {reviewedSuggestions.length > 0 && (
          <>
            <h3 className="mb-4 mt-8 text-sm font-medium text-muted-foreground">
              Previously reviewed ({reviewedSuggestions.length})
            </h3>
            <div className="space-y-4">
              {reviewedSuggestions.map((suggestion) => (
                <SuggestionCard
                  key={suggestion.id}
                  suggestion={suggestion}
                  decision={findDecision(curation.decisions, suggestion.id)}
                  onAccept={() => {}}
                  onReject={() => {}}
                />
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
