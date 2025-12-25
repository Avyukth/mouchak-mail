/**
 * TypeScript types for MCP Agent Mail API responses
 * Matches backend Rust types in lib-core
 */

// ============================================================================
// Core Types
// ============================================================================

export interface Project {
    id: number;
    slug: string;
    human_key: string;
    created_at: string;
    agent_count?: number;
    message_count?: number;
}

export interface Agent {
    id: number;
    project_id: number;
    name: string;
    program: string;
    model: string;
    task_description: string;
    inception_ts: string;
    last_active_ts: string;
    avatar_emoji?: string;
    contact_policy?: string;
}

export interface Message {
    id: number;
    project_id: number;
    project_slug?: string;
    sender_id: number;
    sender_name?: string;
    thread_id: string | null;
    subject: string;
    body_md: string;
    importance: 'low' | 'normal' | 'high' | 'urgent';
    ack_required: boolean;
    created_ts: string;
    created_relative?: string;
    excerpt?: string;
    recipient_names?: string[];
    recipients?: string[];
    is_read?: boolean;
    acknowledged_at?: string | null;
}

// ============================================================================
// Thread Types
// ============================================================================

export interface Thread {
    thread_id: string;
    subject: string;
    messages: Message[];
    participants: string[];
    message_count: number;
    last_message_ts: string;
}

export interface ThreadSummary {
    thread_id: string;
    subject: string;
    message_count: number;
    participants: string[];
    first_message_ts: string;
    last_message_ts: string;
}

// ============================================================================
// File Reservation Types
// ============================================================================

export interface FileReservation {
    id: number;
    project_id: number;
    agent_id: number;
    agent_name?: string;
    path_pattern: string;
    exclusive: boolean;
    expires_ts: string;
    reason?: string;
    created_at: string;
}

export interface FileReservationGrant {
    granted: FileReservation[];
    conflicts: FileReservation[];
}

// ============================================================================
// Attachment Types
// ============================================================================

export interface Attachment {
    id: number;
    message_id: number;
    filename: string;
    mime_type: string;
    size_bytes: number;
    created_at: string;
}

// ============================================================================
// Unified Inbox Types
// ============================================================================

export interface UnifiedInboxMessage extends Message {
    project_slug: string;
    sender_name: string;
    recipient_names: string[];
    created_relative: string;
    excerpt: string;
}

export interface UnifiedInboxResponse {
    messages: UnifiedInboxMessage[];
    total_count: number;
}

// ============================================================================
// Activity Types
// ============================================================================

export interface ActivityItem {
    id: number;
    project_id: number;
    activity_type: string;
    actor_name?: string;
    subject?: string;
    details?: string;
    created_ts: string;
}

// ============================================================================
// Archive Types
// ============================================================================

export interface ArchiveCommit {
    sha: string;
    message: string;
    timestamp: string;
    author: string;
}

export interface ArchiveFile {
    path: string;
    size_bytes: number;
}

// ============================================================================
// Build Slot Types
// ============================================================================

export interface BuildSlot {
    id: number;
    project_id: number;
    agent_id: number;
    agent_name?: string;
    slot_type: string;
    acquired_at: string;
    expires_at: string;
    reason?: string;
}

// ============================================================================
// Metrics Types
// ============================================================================

export interface ToolMetric {
    tool_name: string;
    call_count: number;
    error_count: number;
    avg_duration_ms: number;
    last_called_at?: string;
}

export interface ToolStats {
    total_calls: number;
    total_errors: number;
    avg_duration_ms: number;
    top_tools: ToolMetric[];
}

// ============================================================================
// Error Types
// ============================================================================

export interface ApiError {
    error: string;
    message: string;
}

// ============================================================================
// Request Types
// ============================================================================

export interface SendMessageRequest {
    project_slug: string;
    sender_name: string;
    recipient_names: string[];
    subject: string;
    body_md: string;
    thread_id?: string;
    importance?: 'low' | 'normal' | 'high' | 'urgent';
    ack_required?: boolean;
    cc_names?: string[];
    bcc_names?: string[];
}

export interface SearchMessagesRequest {
    project_slug: string;
    query: string;
    limit?: number;
}

export interface FileReservationRequest {
    project_slug: string;
    agent_name: string;
    paths: string[];
    ttl_seconds?: number;
    exclusive?: boolean;
    reason?: string;
}
