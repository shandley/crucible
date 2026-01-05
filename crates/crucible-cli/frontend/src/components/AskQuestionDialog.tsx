import { useState, useEffect } from 'react'
import { askQuestion, getObservationExplanation } from '../api/client'
import type { AskQuestionResponse, ObservationExplanation } from '../types'
import { Button } from './Button'

interface AskQuestionDialogProps {
  observationId?: string
  suggestionId?: string
  onClose: () => void
}

export function AskQuestionDialog({
  observationId,
  suggestionId,
  onClose,
}: AskQuestionDialogProps) {
  const [question, setQuestion] = useState('')
  const [loading, setLoading] = useState(false)
  const [response, setResponse] = useState<AskQuestionResponse | null>(null)
  const [explanation, setExplanation] = useState<ObservationExplanation | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Load initial explanation when dialog opens
  const loadExplanation = async () => {
    if (!observationId) return
    setLoading(true)
    setError(null)
    try {
      const result = await getObservationExplanation(observationId)
      setExplanation(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load explanation')
    } finally {
      setLoading(false)
    }
  }

  // Load explanation on mount
  useEffect(() => {
    loadExplanation()
  }, [])

  const handleAsk = async (questionText?: string) => {
    const q = questionText || question
    if (!q.trim()) return

    setLoading(true)
    setError(null)
    try {
      const result = await askQuestion({
        question: q,
        observation_id: observationId,
        suggestion_id: suggestionId,
      })
      setResponse(result)
      setQuestion('')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get answer')
    } finally {
      setLoading(false)
    }
  }

  const handleFollowUp = (q: string) => {
    handleAsk(q)
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="px-6 py-4 border-b flex justify-between items-center">
          <h2 className="text-lg font-semibold">Ask About This Issue</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {/* Explanation Card */}
          {explanation && (
            <div className="bg-blue-50 rounded-lg p-4">
              <h3 className="font-medium text-blue-900 mb-2">Explanation</h3>
              <p className="text-blue-800 text-sm">{explanation.explanation}</p>

              {/* Calibrated Confidence */}
              <div className="mt-3 flex items-center gap-4 text-sm">
                <div className="flex items-center gap-2">
                  <span className="text-blue-600">Original:</span>
                  <span className="font-medium">{explanation.original_confidence.toFixed(0)}%</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-blue-600">Calibrated:</span>
                  <span className="font-medium text-blue-900">
                    {explanation.calibrated_confidence.toFixed(0)}%
                  </span>
                </div>
              </div>

              <p className="text-blue-700 text-xs mt-2 italic">
                {explanation.calibration_reasoning}
              </p>
            </div>
          )}

          {/* Previous Response */}
          {response && (
            <div className="bg-green-50 rounded-lg p-4">
              <h3 className="font-medium text-green-900 mb-2">Answer</h3>
              <p className="text-green-800 text-sm">{response.answer}</p>
              <div className="mt-2 text-xs text-green-600">
                Confidence: {response.confidence.toFixed(0)}%
              </div>

              {/* Follow-up questions */}
              {response.follow_up_questions.length > 0 && (
                <div className="mt-3">
                  <span className="text-xs text-green-700 font-medium">Suggested follow-ups:</span>
                  <div className="mt-1 flex flex-wrap gap-2">
                    {response.follow_up_questions.map((q, i) => (
                      <button
                        key={i}
                        onClick={() => handleFollowUp(q)}
                        disabled={loading}
                        className="text-xs bg-green-100 hover:bg-green-200 text-green-800 px-2 py-1 rounded disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {q}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="bg-red-50 text-red-700 rounded-lg p-4 text-sm">
              {error}
            </div>
          )}
        </div>

        {/* Question Input */}
        <div className="px-6 py-4 border-t bg-gray-50">
          <div className="flex gap-2">
            <input
              type="text"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleAsk()}
              placeholder="Ask a question about this issue..."
              className="flex-1 px-3 py-2 border rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              disabled={loading}
            />
            <Button
              onClick={() => handleAsk()}
              disabled={loading || !question.trim()}
            >
              {loading ? 'Asking...' : 'Ask'}
            </Button>
          </div>

          {/* Quick questions */}
          {!response && explanation?.suggested_questions && (
            <div className="mt-2 flex flex-wrap gap-2">
              {explanation.suggested_questions.map((q, i) => (
                <button
                  key={i}
                  onClick={() => handleFollowUp(q)}
                  disabled={loading}
                  className="text-xs bg-gray-200 hover:bg-gray-300 text-gray-700 px-2 py-1 rounded disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {q}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
