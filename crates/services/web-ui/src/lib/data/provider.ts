/**
 * DataProvider Interface
 *
 * Abstracts data fetching for build-time selection between:
 * - API Provider (embedded UI - fetches from /api/*)
 * - Static Provider (GitHub Pages - reads from bundled JSON)
 *
 * This enables complete build isolation via Vite tree-shaking.
 */

import type {
	Project,
	Agent,
	Message,
	Thread,
	ThreadSummary,
	UnifiedInboxResponse,
	ActivityItem,
	ArchiveCommit,
	ArchiveFile,
	ToolMetric,
	ToolStats,
	FileReservation,
	Attachment
} from '$lib/api/types';

/**
 * Dashboard statistics aggregated from multiple sources
 */
export interface DashboardStats {
	projectCount: number;
	agentCount: number;
	inboxCount: number;
	messageCount: number;
	projects: Array<Project & { agentCount?: number }>;
}

/**
 * Static data metadata (only present in static mode)
 */
export interface StaticDataMeta {
	exportedAt: string;
	version: string;
	mode: 'static' | 'api';
}

/**
 * Core data provider interface
 *
 * Implementations:
 * - ApiProvider: Real-time fetch from backend API
 * - StaticProvider: Pre-bundled JSON for offline/GitHub Pages
 */
export interface DataProvider {
	// ============================================================================
	// Metadata
	// ============================================================================

	/** Get provider mode and metadata */
	getMeta(): Promise<StaticDataMeta>;

	/** Check if backend is available (always false for static) */
	checkHealth(): Promise<{ status: string }>;

	// ============================================================================
	// Dashboard
	// ============================================================================

	/** Get aggregated dashboard statistics */
	getDashboardStats(): Promise<DashboardStats>;

	// ============================================================================
	// Projects
	// ============================================================================

	/** List all projects */
	getProjects(): Promise<Project[]>;

	/** Get projects with agent/message counts */
	getProjectsWithStats(): Promise<Project[]>;

	/** Get single project by slug */
	getProjectInfo(projectSlug: string): Promise<Project>;

	// ============================================================================
	// Agents
	// ============================================================================

	/** List agents for a project */
	getAgents(projectSlug: string): Promise<Agent[]>;

	/** Get agent profile */
	getAgentProfile(projectSlug: string, agentName: string): Promise<Agent>;

	// ============================================================================
	// Messages
	// ============================================================================

	/** Get inbox for an agent */
	getInbox(projectSlug: string, agentName: string): Promise<Message[]>;

	/** Get outbox for an agent */
	getOutbox(projectSlug: string, agentName: string): Promise<Message[]>;

	/** Get single message by ID */
	getMessage(id: number): Promise<Message>;

	/** Search messages */
	searchMessages(projectSlug: string, query: string, limit?: number): Promise<Message[]>;

	// ============================================================================
	// Unified Inbox
	// ============================================================================

	/** Get unified inbox across all projects */
	fetchUnifiedInbox(limit?: number): Promise<UnifiedInboxResponse>;

	// ============================================================================
	// Threads
	// ============================================================================

	/** Get thread by ID */
	getThread(projectSlug: string, threadId: string): Promise<Thread>;

	/** List thread summaries for a project */
	listThreads(projectSlug: string): Promise<ThreadSummary[]>;

	// ============================================================================
	// Activity
	// ============================================================================

	/** List activity items */
	listActivity(projectSlug?: string, limit?: number): Promise<ActivityItem[]>;

	// ============================================================================
	// Metrics
	// ============================================================================

	/** List tool metrics */
	listToolMetrics(): Promise<ToolMetric[]>;

	/** Get aggregated tool stats */
	getToolStats(): Promise<ToolStats>;

	// ============================================================================
	// Archive
	// ============================================================================

	/** List archive commits */
	listArchiveCommits(limit?: number): Promise<ArchiveCommit[]>;

	/** Get archive commit details */
	getArchiveCommit(sha: string): Promise<ArchiveCommit>;

	/** List files in an archive commit */
	listArchiveFiles(sha: string): Promise<ArchiveFile[]>;

	/** Get file content from archive */
	getArchiveFileContent(sha: string, path: string): Promise<string>;

	// ============================================================================
	// File Reservations (Read-only)
	// ============================================================================

	/** List file reservations for a project */
	listFileReservations(projectSlug: string): Promise<FileReservation[]>;

	// ============================================================================
	// Attachments (Read-only)
	// ============================================================================

	/** List attachments */
	listAttachments(projectSlug?: string): Promise<Attachment[]>;

	// ============================================================================
	// Message Actions
	// ============================================================================

	/** Mark a message as read (no-op in static mode) */
	markMessageRead(
		projectSlug: string,
		agentName: string,
		messageId: number
	): Promise<{ marked: boolean; message_id: number }>;
}

/**
 * Check if we're in static mode (compile-time constant for tree-shaking)
 */
export function isStaticMode(): boolean {
	return import.meta.env.VITE_DATA_MODE === 'static';
}
