export type Page = 'Assistant' | 'Conversations' | 'Tools' | 'System'
export type ModelState = 'online' | 'offline' | 'loading' | 'misconfigured'
export interface Status {
  backend: 'online'
  model_server: ModelState
  model: string
  diagnostic: string
  version: string
  context_tokens: number
  max_output_tokens: number
}
export interface Summary {
  id: string
  title: string
  created_at: string
  updated_at: string
}
export interface Activity {
  call_id: string
  name: string
  ok: boolean | null
  result: Record<string, unknown>
}
export interface Turn {
  seq: number
  id: string
  user: string
  assistant: string
  created_at: string
  tools: Activity[]
  status: 'generating' | 'complete' | 'interrupted' | 'error'
  error: string | null
}
export interface Detail extends Summary {
  turns: Turn[]
  next_before: number | null
}
export interface ToolInfo {
  name: string
  description: string
  permission: string
  parameters: object
}
export interface Outcome {
  ok: boolean
  result: Record<string, unknown>
}
export type ChatEvent =
  | { type: 'message_start'; turn_id: string; conversation_id: string }
  | { type: 'text_delta'; text: string }
  | { type: 'tool_start'; call_id: string; name: string }
  | ({ type: 'tool_result' } & Activity)
  | { type: 'message_complete'; turn_id: string }
  | { type: 'error'; code: string; message: string }
