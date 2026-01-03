import { useState, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getCuration, getDataPreview, acceptDecision, rejectDecision, saveCuration, batchAccept, batchReject } from './api/client'
import type { BatchRequest } from './api/client'
import { SuggestionCard, StatusBar, Button, DataPreview } from './components'
import type { DecisionInfo, SuggestionInfo, ObservationInfo, DataPreviewResponse } from './types'

function findDecision(decisions: DecisionInfo[], suggestionId: string): DecisionInfo | undefined {
  return decisions.find(d => d.suggestion_id === suggestionId)
}

function findObservation(observations: ObservationInfo[], observationId: string): ObservationInfo | undefined {
  return observations.find(o => o.id === observationId)
}

/** Calculate affected rows from value_counts evidence by finding rows with matching values */
function calculateAffectedRows(
  observation: ObservationInfo,
  data: DataPreviewResponse | undefined
): number[] {
  if (!data) return []

  const colIndex = data.headers.indexOf(observation.column)
  if (colIndex === -1) return []

  const valueCounts = observation.evidence?.value_counts
  if (!valueCounts) return []

  // Collect all variant values from value_counts
  // Two formats to handle:
  // 1. Case variant: { "canonical": { "Variant1": count, "Variant2": count } }
  // 2. Typo: { "typoValue": { "count": N, "suggestion": "correct" } }
  const targetValues = new Set<string>()

  for (const [key, value] of Object.entries(valueCounts)) {
    if (typeof value === 'object' && value !== null) {
      const innerKeys = Object.keys(value)
      // Check if this is typo format (has "count" and "suggestion" keys)
      if (innerKeys.includes('count') || innerKeys.includes('suggestion')) {
        // Typo format: the outer key is the typo value
        targetValues.add(key)
      } else {
        // Case variant format: inner keys are the variant values
        for (const variant of innerKeys) {
          targetValues.add(variant)
        }
      }
    }
  }

  // Find rows containing these values
  const affectedRows: number[] = []
  data.rows.forEach((row, rowIndex) => {
    const cellValue = row[colIndex]
    if (targetValues.has(cellValue)) {
      affectedRows.push(rowIndex)
    }
  })

  return affectedRows
}

function isPending(decisions: DecisionInfo[], suggestion: SuggestionInfo): boolean {
  const decision = findDecision(decisions, suggestion.id)
  return !decision || decision.status === 'pending'
}

export default function App() {
  const queryClient = useQueryClient()
  const [selectedSuggestionId, setSelectedSuggestionId] = useState<string | null>(null)

  const { data: curation, isLoading, error } = useQuery({
    queryKey: ['curation'],
    queryFn: getCuration,
  })

  const { data: dataPreview } = useQuery({
    queryKey: ['data-preview'],
    queryFn: getDataPreview,
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

  const batchAcceptMutation = useMutation({
    mutationFn: (request: BatchRequest) => batchAccept(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
    },
  })

  const batchRejectMutation = useMutation({
    mutationFn: (request: BatchRequest) => batchReject(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
    },
  })

  // Get highlighted rows and column based on selected suggestion
  const { highlightedRows, highlightedColumn } = useMemo(() => {
    if (!curation || !selectedSuggestionId) {
      return { highlightedRows: [], highlightedColumn: undefined }
    }

    const suggestion = curation.suggestions.find(s => s.id === selectedSuggestionId)
    if (!suggestion) {
      return { highlightedRows: [], highlightedColumn: undefined }
    }

    const observation = findObservation(curation.observations, suggestion.observation_id)
    if (!observation) {
      return { highlightedRows: [], highlightedColumn: undefined }
    }

    // Use sample_rows if available, otherwise calculate from value_counts
    let rows = observation.evidence?.sample_rows
    if (!rows || rows.length === 0) {
      rows = calculateAffectedRows(observation, dataPreview)
    }

    return {
      highlightedRows: rows,
      highlightedColumn: observation.column,
    }
  }, [curation, selectedSuggestionId, dataPreview])

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
    <div className="flex h-screen flex-col bg-muted/30">
      <StatusBar
        summary={curation.summary}
        filename={curation.source.file}
        progress={curation.progress}
      />

      <div className="flex flex-1 overflow-hidden">
        {/* Left panel: Suggestions */}
        <div className="flex w-1/2 flex-col border-r">
          <div className="flex items-center justify-between border-b bg-background px-4 py-3">
            <h2 className="text-sm font-medium">
              {curation.summary.pending_count > 0
                ? `${curation.summary.pending_count} pending review`
                : 'All suggestions reviewed'}
            </h2>
            <div className="flex items-center gap-2">
              {curation.summary.pending_count > 0 && (
                <>
                  <Button
                    variant="default"
                    size="sm"
                    onClick={() => batchAcceptMutation.mutate({ all: true })}
                    disabled={batchAcceptMutation.isPending}
                  >
                    {batchAcceptMutation.isPending ? 'Accepting...' : 'Accept All'}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => batchRejectMutation.mutate({ all: true, notes: 'Batch rejected' })}
                    disabled={batchRejectMutation.isPending}
                  >
                    {batchRejectMutation.isPending ? 'Rejecting...' : 'Reject All'}
                  </Button>
                </>
              )}
              <Button
                variant="secondary"
                size="sm"
                onClick={() => saveMutation.mutate()}
                disabled={saveMutation.isPending}
              >
                {saveMutation.isPending ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </div>

          <div className="flex-1 overflow-auto p-4">
            {curation.summary.pending_count === 0 && (
              <div className="mb-4 rounded-lg border border-success/50 bg-success/5 p-4 text-center">
                <p className="font-medium text-success">Review complete!</p>
                <p className="mt-1 text-sm text-muted-foreground">
                  Run <code className="rounded bg-muted px-1">crucible apply</code> to
                  generate your curated dataset.
                </p>
              </div>
            )}

            <div className="space-y-3">
              {pendingSuggestions.map((suggestion) => (
                <div
                  key={suggestion.id}
                  onClick={() => setSelectedSuggestionId(
                    selectedSuggestionId === suggestion.id ? null : suggestion.id
                  )}
                  className="cursor-pointer"
                >
                  <SuggestionCard
                    suggestion={suggestion}
                    decision={findDecision(curation.decisions, suggestion.id)}
                    isSelected={selectedSuggestionId === suggestion.id}
                    onAccept={(notes) =>
                      acceptMutation.mutate({ id: suggestion.id, notes })
                    }
                    onReject={(notes) =>
                      rejectMutation.mutate({ id: suggestion.id, notes })
                    }
                  />
                </div>
              ))}
            </div>

            {reviewedSuggestions.length > 0 && (
              <>
                <h3 className="mb-3 mt-6 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  Previously reviewed ({reviewedSuggestions.length})
                </h3>
                <div className="space-y-3">
                  {reviewedSuggestions.map((suggestion) => (
                    <div
                      key={suggestion.id}
                      onClick={() => setSelectedSuggestionId(
                        selectedSuggestionId === suggestion.id ? null : suggestion.id
                      )}
                      className="cursor-pointer"
                    >
                      <SuggestionCard
                        suggestion={suggestion}
                        decision={findDecision(curation.decisions, suggestion.id)}
                        isSelected={selectedSuggestionId === suggestion.id}
                        onAccept={() => {}}
                        onReject={() => {}}
                      />
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        </div>

        {/* Right panel: Data Preview */}
        <div className="flex w-1/2 flex-col bg-background">
          <DataPreview
            highlightedRows={highlightedRows}
            highlightedColumn={highlightedColumn}
          />
        </div>
      </div>
    </div>
  )
}
