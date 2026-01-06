import type {
  AskQuestionRequest,
  AskQuestionResponse,
  CalibrateConfidenceRequest,
  CalibrateConfidenceResponse,
  CurationResponse,
  DataPreviewParams,
  DataPreviewResponse,
  DecisionResponse,
  ObservationExplanation,
  SaveResponse,
} from '../types'

const API_BASE = '/api'

async function fetchApi<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    headers: {
      'Content-Type': 'application/json',
    },
    ...options,
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(error || `HTTP ${response.status}`)
  }

  return response.json()
}

export async function getCuration(): Promise<CurationResponse> {
  return fetchApi<CurationResponse>('/curation')
}

export async function acceptDecision(
  id: string,
  notes?: string
): Promise<DecisionResponse> {
  return fetchApi<DecisionResponse>(`/decisions/${id}/accept`, {
    method: 'POST',
    body: JSON.stringify({ notes }),
  })
}

export async function rejectDecision(
  id: string,
  notes: string
): Promise<DecisionResponse> {
  return fetchApi<DecisionResponse>(`/decisions/${id}/reject`, {
    method: 'POST',
    body: JSON.stringify({ notes }),
  })
}

export async function modifyDecision(
  id: string,
  modifications: unknown,
  notes: string
): Promise<DecisionResponse> {
  return fetchApi<DecisionResponse>(`/decisions/${id}/modify`, {
    method: 'POST',
    body: JSON.stringify({ modifications, notes }),
  })
}

export interface ResetResponse {
  suggestion_id: string
  was_reset: boolean
  previous_status: string | null
}

export async function resetDecision(id: string): Promise<ResetResponse> {
  return fetchApi<ResetResponse>(`/decisions/${id}/reset`, {
    method: 'POST',
  })
}

export async function saveCuration(): Promise<SaveResponse> {
  return fetchApi<SaveResponse>('/save', { method: 'POST' })
}

export async function getDataPreview(
  params?: DataPreviewParams
): Promise<DataPreviewResponse> {
  const queryParams = new URLSearchParams()
  if (params?.offset !== undefined) {
    queryParams.set('offset', String(params.offset))
  }
  if (params?.limit !== undefined) {
    queryParams.set('limit', String(params.limit))
  }
  const queryString = queryParams.toString()
  const url = queryString ? `/data?${queryString}` : '/data'
  return fetchApi<DataPreviewResponse>(url)
}

export interface BatchRequest {
  action_type?: string
  column?: string
  all?: boolean
  user?: string
  notes?: string
}

export interface BatchResponse {
  processed: number
  remaining: number
  decisions: DecisionResponse[]
}

export async function batchAccept(request: BatchRequest): Promise<BatchResponse> {
  return fetchApi<BatchResponse>('/batch/accept', {
    method: 'POST',
    body: JSON.stringify(request),
  })
}

export async function batchReject(request: BatchRequest): Promise<BatchResponse> {
  return fetchApi<BatchResponse>('/batch/reject', {
    method: 'POST',
    body: JSON.stringify(request),
  })
}

// Interactive explanation APIs
export async function askQuestion(
  request: AskQuestionRequest
): Promise<AskQuestionResponse> {
  return fetchApi<AskQuestionResponse>('/explain/ask', {
    method: 'POST',
    body: JSON.stringify(request),
  })
}

export async function calibrateConfidence(
  request: CalibrateConfidenceRequest
): Promise<CalibrateConfidenceResponse> {
  return fetchApi<CalibrateConfidenceResponse>('/explain/calibrate', {
    method: 'POST',
    body: JSON.stringify(request),
  })
}

export async function getObservationExplanation(
  observationId: string
): Promise<ObservationExplanation> {
  return fetchApi<ObservationExplanation>(`/explain/observation/${observationId}`)
}

// LLM status
export interface LlmStatusResponse {
  available: boolean
  provider: string | null
  message: string
}

export async function getLlmStatus(): Promise<LlmStatusResponse> {
  return fetchApi<LlmStatusResponse>('/llm/status')
}
