import { useState, useMemo, useCallback, useEffect, useRef } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getCuration, getDataPreview, acceptDecision, rejectDecision, resetDecision, saveCuration, batchAccept, batchReject } from './api/client'
import type { BatchRequest } from './api/client'
import { SuggestionCard, SuggestionGroup, StatusBar, Button, DataPreview } from './components'
import type { DecisionInfo, SuggestionInfo, ObservationInfo, DataPreviewResponse } from './types'

/** An action that can be undone/redone */
interface UndoableAction {
  suggestionId: string
  type: 'accept' | 'reject'
  notes?: string
}

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

/** Format a timestamp as a relative time string */
function formatRelativeTime(isoString: string): string {
  const date = new Date(isoString)
  const now = new Date()
  const diffSeconds = Math.floor((now.getTime() - date.getTime()) / 1000)

  if (diffSeconds < 5) return 'just now'
  if (diffSeconds < 60) return `${diffSeconds}s ago`
  if (diffSeconds < 3600) return `${Math.floor(diffSeconds / 60)}m ago`
  if (diffSeconds < 86400) return `${Math.floor(diffSeconds / 3600)}h ago`
  return date.toLocaleDateString()
}

export default function App() {
  const queryClient = useQueryClient()
  const [selectedSuggestionId, setSelectedSuggestionId] = useState<string | null>(null)
  const [undoStack, setUndoStack] = useState<UndoableAction[]>([])
  const [redoStack, setRedoStack] = useState<UndoableAction[]>([])
  const [columnFilter, setColumnFilter] = useState<string>('all')
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null)
  const [, setTick] = useState(0) // Force re-render for relative time updates
  const [viewMode, setViewMode] = useState<'flat' | 'grouped'>('flat')
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set())
  const suggestionRefs = useRef<Map<string, HTMLDivElement>>(new Map())

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
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
      // Push to undo stack, clear redo stack
      setUndoStack(prev => [...prev, { suggestionId: variables.id, type: 'accept', notes: variables.notes }])
      setRedoStack([])
      // Update saved timestamp (auto-save happens on backend)
      setLastSavedAt(new Date().toISOString())
    },
  })

  const rejectMutation = useMutation({
    mutationFn: ({ id, notes }: { id: string; notes: string }) =>
      rejectDecision(id, notes),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
      // Push to undo stack, clear redo stack
      setUndoStack(prev => [...prev, { suggestionId: variables.id, type: 'reject', notes: variables.notes }])
      setRedoStack([])
      // Update saved timestamp (auto-save happens on backend)
      setLastSavedAt(new Date().toISOString())
    },
  })

  const resetMutation = useMutation({
    mutationFn: (id: string) => resetDecision(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
      // Update saved timestamp (auto-save happens on backend)
      setLastSavedAt(new Date().toISOString())
    },
  })

  const saveMutation = useMutation({
    mutationFn: saveCuration,
    onSuccess: (data) => {
      setLastSavedAt(data.saved_at)
    },
  })

  // Initialize lastSavedAt from curation data
  useEffect(() => {
    if (curation?.updated_at && !lastSavedAt) {
      setLastSavedAt(curation.updated_at)
    }
  }, [curation?.updated_at, lastSavedAt])

  // Update relative time display every 10 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      setTick(t => t + 1)
    }, 10000)
    return () => clearInterval(interval)
  }, [])

  const batchAcceptMutation = useMutation({
    mutationFn: (request: BatchRequest) => batchAccept(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
      // Clear undo/redo stacks for batch operations
      setUndoStack([])
      setRedoStack([])
      // Update saved timestamp (auto-save happens on backend)
      setLastSavedAt(new Date().toISOString())
    },
  })

  const batchRejectMutation = useMutation({
    mutationFn: (request: BatchRequest) => batchReject(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['curation'] })
      // Clear undo/redo stacks for batch operations
      setUndoStack([])
      setRedoStack([])
      // Update saved timestamp (auto-save happens on backend)
      setLastSavedAt(new Date().toISOString())
    },
  })

  // Undo the last action
  const handleUndo = useCallback(async () => {
    if (undoStack.length === 0) return

    const lastAction = undoStack[undoStack.length - 1]

    // Reset the decision
    await resetMutation.mutateAsync(lastAction.suggestionId)

    // Move from undo to redo stack
    setUndoStack(prev => prev.slice(0, -1))
    setRedoStack(prev => [...prev, lastAction])
  }, [undoStack, resetMutation])

  // Redo the last undone action
  const handleRedo = useCallback(async () => {
    if (redoStack.length === 0) return

    const lastAction = redoStack[redoStack.length - 1]

    // Re-apply the decision
    if (lastAction.type === 'accept') {
      await acceptDecision(lastAction.suggestionId, lastAction.notes)
    } else {
      await rejectDecision(lastAction.suggestionId, lastAction.notes || 'Rejected')
    }

    queryClient.invalidateQueries({ queryKey: ['curation'] })

    // Move from redo to undo stack
    setRedoStack(prev => prev.slice(0, -1))
    setUndoStack(prev => [...prev, lastAction])
  }, [redoStack, queryClient])

  // Toggle group expansion
  const toggleGroup = useCallback((column: string) => {
    setExpandedGroups(prev => {
      const next = new Set(prev)
      if (next.has(column)) {
        next.delete(column)
      } else {
        next.add(column)
      }
      return next
    })
  }, [])

  // Extract unique columns from observations for filtering
  const availableColumns = useMemo(() => {
    if (!curation) return []
    const columns = new Set<string>()
    for (const obs of curation.observations) {
      if (obs.column) {
        columns.add(obs.column)
      }
    }
    return Array.from(columns).sort()
  }, [curation])

  // Helper to get column for a suggestion
  const getColumnForSuggestion = useCallback((suggestion: SuggestionInfo): string | undefined => {
    if (!curation) return undefined
    const observation = findObservation(curation.observations, suggestion.observation_id)
    return observation?.column
  }, [curation])

  // Group suggestions by column
  interface SuggestionGroupData {
    column: string
    suggestions: SuggestionInfo[]
    pendingCount: number
  }

  const groupedSuggestions = useMemo((): SuggestionGroupData[] => {
    if (!curation) return []

    const groups = new Map<string, SuggestionInfo[]>()

    for (const suggestion of curation.suggestions) {
      const column = getColumnForSuggestion(suggestion) || 'unknown'
      if (!groups.has(column)) {
        groups.set(column, [])
      }
      groups.get(column)!.push(suggestion)
    }

    return Array.from(groups.entries())
      .map(([column, suggestions]) => ({
        column,
        suggestions: suggestions.sort((a, b) => a.priority - b.priority),
        pendingCount: suggestions.filter(s => isPending(curation.decisions, s)).length,
      }))
      .sort((a, b) => b.pendingCount - a.pendingCount) // Groups with pending items first
  }, [curation, getColumnForSuggestion])

  // Filtered pending and reviewed suggestions (as useMemo for proper dependency tracking)
  const { pendingSuggestions, reviewedSuggestions } = useMemo(() => {
    if (!curation) return { pendingSuggestions: [], reviewedSuggestions: [] }

    const filterByColumn = (suggestion: SuggestionInfo) => {
      if (columnFilter === 'all') return true
      const col = getColumnForSuggestion(suggestion)
      return col === columnFilter
    }

    const pending = curation.suggestions
      .filter(s => isPending(curation.decisions, s))
      .filter(filterByColumn)
      .sort((a, b) => a.priority - b.priority)

    const reviewed = curation.suggestions
      .filter(s => !isPending(curation.decisions, s))
      .filter(filterByColumn)

    return { pendingSuggestions: pending, reviewedSuggestions: reviewed }
  }, [curation, columnFilter, getColumnForSuggestion])

  // Get visible suggestions in order (for keyboard navigation)
  const visibleSuggestions = useMemo(() => {
    if (!curation) return []

    if (viewMode === 'grouped') {
      // In grouped view, only show suggestions from expanded groups
      const visible: SuggestionInfo[] = []
      for (const group of groupedSuggestions) {
        if (columnFilter !== 'all' && group.column !== columnFilter) continue
        if (expandedGroups.has(group.column)) {
          visible.push(...group.suggestions)
        }
      }
      return visible
    } else {
      // In flat view, show filtered pending suggestions then reviewed
      return [...pendingSuggestions, ...reviewedSuggestions]
    }
  }, [curation, viewMode, groupedSuggestions, expandedGroups, columnFilter, pendingSuggestions, reviewedSuggestions])

  // Navigate to next/previous suggestion
  const navigateToSuggestion = useCallback((direction: 'next' | 'prev') => {
    if (visibleSuggestions.length === 0) return

    const currentIndex = selectedSuggestionId
      ? visibleSuggestions.findIndex(s => s.id === selectedSuggestionId)
      : -1

    let newIndex: number
    if (direction === 'next') {
      newIndex = currentIndex < visibleSuggestions.length - 1 ? currentIndex + 1 : 0
    } else {
      newIndex = currentIndex > 0 ? currentIndex - 1 : visibleSuggestions.length - 1
    }

    const newSuggestion = visibleSuggestions[newIndex]
    if (newSuggestion) {
      setSelectedSuggestionId(newSuggestion.id)
      // Scroll into view
      const element = suggestionRefs.current.get(newSuggestion.id)
      element?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
    }
  }, [visibleSuggestions, selectedSuggestionId])

  // Accept/reject selected suggestion with keyboard
  const acceptSelectedSuggestion = useCallback(() => {
    if (!selectedSuggestionId || !curation) return
    const suggestion = curation.suggestions.find(s => s.id === selectedSuggestionId)
    if (!suggestion) return

    // Only accept if pending
    const decision = findDecision(curation.decisions, suggestion.id)
    if (!decision || decision.status === 'pending') {
      acceptMutation.mutate({ id: suggestion.id })
    }
  }, [selectedSuggestionId, curation, acceptMutation])

  const rejectSelectedSuggestion = useCallback(() => {
    if (!selectedSuggestionId || !curation) return
    const suggestion = curation.suggestions.find(s => s.id === selectedSuggestionId)
    if (!suggestion) return

    // Only reject if pending
    const decision = findDecision(curation.decisions, suggestion.id)
    if (!decision || decision.status === 'pending') {
      rejectMutation.mutate({ id: suggestion.id, notes: 'Rejected via keyboard' })
    }
  }, [selectedSuggestionId, curation, rejectMutation])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if user is typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) {
        return
      }

      // Undo/Redo: Ctrl+Z / Ctrl+Shift+Z
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault()
        if (e.shiftKey) {
          handleRedo()
        } else {
          handleUndo()
        }
        return
      }

      // Navigation: Arrow keys or vim-style j/k
      if (e.key === 'ArrowDown' || e.key === 'j') {
        e.preventDefault()
        navigateToSuggestion('next')
        return
      }
      if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault()
        navigateToSuggestion('prev')
        return
      }

      // Accept: Enter or 'a'
      if (e.key === 'Enter' || e.key === 'a') {
        e.preventDefault()
        acceptSelectedSuggestion()
        return
      }

      // Reject: 'r' or 'x'
      if (e.key === 'r' || e.key === 'x') {
        e.preventDefault()
        rejectSelectedSuggestion()
        return
      }

      // Deselect: Escape
      if (e.key === 'Escape') {
        e.preventDefault()
        setSelectedSuggestionId(null)
        return
      }

      // Toggle view mode: 'g'
      if (e.key === 'g') {
        e.preventDefault()
        setViewMode(v => v === 'flat' ? 'grouped' : 'flat')
        return
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleUndo, handleRedo, navigateToSuggestion, acceptSelectedSuggestion, rejectSelectedSuggestion])

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

  // Count total pending (unfiltered) for header
  const totalPending = curation.suggestions.filter(s => isPending(curation.decisions, s)).length

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
            <div className="flex items-center gap-3">
              <h2 className="text-sm font-medium">
                {totalPending > 0
                  ? columnFilter !== 'all'
                    ? `${pendingSuggestions.length} of ${totalPending} pending`
                    : `${totalPending} pending review`
                  : 'All suggestions reviewed'}
              </h2>
              {availableColumns.length > 1 && (
                <>
                  <select
                    value={columnFilter}
                    onChange={(e) => setColumnFilter(e.target.value)}
                    className="rounded border border-border bg-background px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
                  >
                    <option value="all">All columns</option>
                    {availableColumns.map((col) => (
                      <option key={col} value={col}>
                        {col}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => setViewMode(viewMode === 'flat' ? 'grouped' : 'flat')}
                    className={`rounded px-2 py-1 text-xs transition-colors ${
                      viewMode === 'grouped'
                        ? 'bg-foreground text-background'
                        : 'bg-muted text-muted-foreground hover:bg-muted/80'
                    }`}
                    title={viewMode === 'grouped' ? 'Switch to flat view' : 'Switch to grouped view'}
                  >
                    {viewMode === 'grouped' ? 'Grouped' : 'Group'}
                  </button>
                </>
              )}
            </div>
            <div className="flex items-center gap-2">
              {/* Undo/Redo buttons */}
              <Button
                variant="ghost"
                size="sm"
                onClick={handleUndo}
                disabled={undoStack.length === 0 || resetMutation.isPending}
                title="Undo last decision (Ctrl+Z)"
              >
                Undo
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleRedo}
                disabled={redoStack.length === 0}
                title="Redo last undone decision (Ctrl+Shift+Z)"
              >
                Redo
              </Button>
              <div className="mx-1 h-4 w-px bg-border" />
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
                    variant="destructive"
                    size="sm"
                    onClick={() => batchRejectMutation.mutate({ all: true, notes: 'Batch rejected' })}
                    disabled={batchRejectMutation.isPending}
                  >
                    {batchRejectMutation.isPending ? 'Rejecting...' : 'Reject All'}
                  </Button>
                </>
              )}
              <div className="flex items-center gap-2">
                {lastSavedAt && (
                  <span className="text-xs text-muted-foreground" title={`Last saved: ${new Date(lastSavedAt).toLocaleString()}`}>
                    Saved {formatRelativeTime(lastSavedAt)}
                  </span>
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

            {viewMode === 'grouped' ? (
              <div className="space-y-3">
                {groupedSuggestions
                  .filter(group => columnFilter === 'all' || group.column === columnFilter)
                  .map((group) => (
                    <SuggestionGroup
                      key={group.column}
                      column={group.column}
                      pendingCount={group.pendingCount}
                      totalCount={group.suggestions.length}
                      isExpanded={expandedGroups.has(group.column)}
                      onToggle={() => toggleGroup(group.column)}
                      onAcceptAll={() => batchAcceptMutation.mutate({ column: group.column })}
                      onRejectAll={() => batchRejectMutation.mutate({ column: group.column, notes: 'Batch rejected' })}
                      isAccepting={batchAcceptMutation.isPending}
                      isRejecting={batchRejectMutation.isPending}
                    >
                      {group.suggestions.map((suggestion) => (
                        <div
                          key={suggestion.id}
                          ref={(el) => {
                            if (el) suggestionRefs.current.set(suggestion.id, el)
                            else suggestionRefs.current.delete(suggestion.id)
                          }}
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
                    </SuggestionGroup>
                  ))}
              </div>
            ) : (
              <div className="space-y-3">
                {pendingSuggestions.map((suggestion) => (
                  <div
                    key={suggestion.id}
                    ref={(el) => {
                      if (el) suggestionRefs.current.set(suggestion.id, el)
                      else suggestionRefs.current.delete(suggestion.id)
                    }}
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
            )}

            {reviewedSuggestions.length > 0 && (
              <>
                <h3 className="mb-3 mt-6 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  Previously reviewed ({reviewedSuggestions.length})
                </h3>
                <div className="space-y-3">
                  {reviewedSuggestions.map((suggestion) => (
                    <div
                      key={suggestion.id}
                      ref={(el) => {
                        if (el) suggestionRefs.current.set(suggestion.id, el)
                        else suggestionRefs.current.delete(suggestion.id)
                      }}
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
