// Types matching the actual API responses

export interface SourceInfo {
  file: string
  format: string
  row_count: number
  column_count: number
}

export interface ColumnInfo {
  name: string
  inferred_type: string
  semantic_role: string
  nullable: boolean
  unique: boolean
}

export interface SchemaInfo {
  columns: ColumnInfo[]
}

export interface ObservationEvidence {
  sample_rows?: number[]
  occurrences?: number
  percentage?: number
  value_counts?: Record<string, Record<string, number>>
  pattern?: string
  expected?: Record<string, unknown>
}

export interface ObservationInfo {
  id: string
  type: string
  severity: 'error' | 'warning' | 'info'
  column: string
  description: string
  confidence: number
  evidence: ObservationEvidence
}

export interface SuggestionInfo {
  id: string
  observation_id: string
  action: string
  priority: number
  rationale: string
  affected_rows: number
  confidence: number
  parameters: unknown
}

export interface DecisionInfo {
  id: string
  suggestion_id: string
  status: 'pending' | 'accepted' | 'rejected' | 'modified'
  decided_by: string | null
  decided_at: string | null
  notes: string | null
}

export interface ObservationCounts {
  error: number
  warning: number
  info: number
}

export interface SummaryInfo {
  total_columns: number
  columns_with_issues: number
  total_observations: number
  observations_by_severity: ObservationCounts
  data_quality_score: number
  total_suggestions: number
  pending_count: number
  accepted_count: number
  rejected_count: number
}

export interface CurationResponse {
  source: SourceInfo
  schema: SchemaInfo
  observations: ObservationInfo[]
  suggestions: SuggestionInfo[]
  decisions: DecisionInfo[]
  summary: SummaryInfo
  progress: number
  updated_at: string
}

export interface DecisionResponse {
  id: string
  suggestion_id: string
  status: string
  decided_by: string | null
  decided_at: string | null
  notes: string | null
}

export interface SaveResponse {
  success: boolean
  path: string
  saved_at: string
}

export interface DataPreviewResponse {
  headers: string[]
  rows: string[][]
  total_rows: number
  truncated: boolean
}

// Interactive explanation types
export interface AskQuestionRequest {
  question: string
  observation_id?: string
  suggestion_id?: string
}

export interface AskQuestionResponse {
  answer: string
  confidence: number
  follow_up_questions: string[]
}

export interface CalibrateConfidenceRequest {
  observation_id: string
}

export interface ConfidenceFactorInfo {
  name: string
  impact: number
  explanation: string
}

export interface CalibrateConfidenceResponse {
  observation_id: string
  original_confidence: number
  calibrated_confidence: number
  reasoning: string
  factors: ConfidenceFactorInfo[]
}

export interface ObservationExplanation {
  observation_id: string
  explanation: string
  original_confidence: number
  calibrated_confidence: number
  calibration_reasoning: string
  suggested_questions: string[]
}
