import type { CurationResponse, DecisionResponse, SaveResponse } from '../types'

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

export async function saveCuration(): Promise<SaveResponse> {
  return fetchApi<SaveResponse>('/save', { method: 'POST' })
}
