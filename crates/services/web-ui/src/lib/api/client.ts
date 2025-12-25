/**
 * API Client for MCP Agent Mail backend
 * Expanded with all endpoints for SVELTE-008
 */

import type {
	Project,
	Agent,
	Message,
	Thread,
	ThreadSummary,
	FileReservation,
	FileReservationGrant,
	Attachment,
	UnifiedInboxResponse,
	ActivityItem,
	ArchiveCommit,
	ArchiveFile,
	BuildSlot,
	ToolMetric,
	ToolStats,
	ApiError,
	SendMessageRequest,
	SearchMessagesRequest,
	FileReservationRequest
} from './types';

const API_BASE = '/api';
const DEFAULT_TIMEOUT = 30000; // 30 seconds

// ============================================================================
// Request Helper
// ============================================================================

class ApiRequestError extends Error {
	constructor(
		public statusCode: number,
		public apiError: ApiError
	) {
		super(apiError.message || 'API request failed');
		this.name = 'ApiRequestError';
	}
}

async function request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
	const url = `${API_BASE}${endpoint}`;
	const controller = new AbortController();
	const timeoutId = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT);

	try {
		const response = await fetch(url, {
			headers: {
				'Content-Type': 'application/json',
				...options.headers
			},
			signal: controller.signal,
			...options
		});

		if (!response.ok) {
			const error: ApiError = await response.json().catch(() => ({
				error: 'unknown',
				message: response.statusText
			}));
			throw new ApiRequestError(response.status, error);
		}

		// Handle empty responses
		const text = await response.text();
		if (!text) {
			return {} as T;
		}
		return JSON.parse(text);
	} finally {
		clearTimeout(timeoutId);
	}
}

// ============================================================================
// Projects
// ============================================================================

export async function getProjects(): Promise<Project[]> {
	return request<Project[]>('/projects');
}

export async function ensureProject(humanKey: string): Promise<Project> {
	return request<Project>('/project/ensure', {
		method: 'POST',
		body: JSON.stringify({ human_key: humanKey })
	});
}

export async function getProjectInfo(projectSlug: string): Promise<Project> {
	return request<Project>('/project/info', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug })
	});
}

export async function deleteProject(projectSlug: string): Promise<{ success: boolean; message: string }> {
	return request<{ success: boolean; message: string }>(`/projects/${projectSlug}`, {
		method: 'DELETE'
	});
}

interface ProjectInfoResponse {
	id: number;
	slug: string;
	human_key: string;
	created_at: string;
	agent_count: number;
	message_count: number;
}

export async function getProjectsWithStats(): Promise<Project[]> {
	const projects = await getProjects();
	
	const statsPromises = projects.map(p =>
		request<ProjectInfoResponse>('/project/info', {
			method: 'POST',
			body: JSON.stringify({ project_slug: p.slug })
		}).catch(() => ({ agent_count: 0, message_count: 0 }))
	);
	
	const stats = await Promise.all(statsPromises);
	
	return projects.map((p, i) => ({
		...p,
		agent_count: stats[i]?.agent_count ?? 0,
		message_count: stats[i]?.message_count ?? 0
	}));
}

// ============================================================================
// Agents
// ============================================================================

export async function getAgents(projectSlug: string): Promise<Agent[]> {
	return request<Agent[]>(`/projects/${projectSlug}/agents`);
}

export async function registerAgent(
	projectSlug: string,
	name: string,
	program: string,
	model: string,
	taskDescription: string
): Promise<Agent> {
	return request<Agent>('/agent/register', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			name,
			program,
			model,
			task_description: taskDescription
		})
	});
}

export async function getAgentProfile(projectSlug: string, agentName: string): Promise<Agent> {
	return request<Agent>('/agent/profile', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug, agent_name: agentName })
	});
}

export async function whois(projectSlug: string, agentName: string): Promise<Agent> {
	return request<Agent>('/agent/whois', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug, agent_name: agentName })
	});
}

export async function deleteAgent(
	projectSlug: string,
	agentName: string
): Promise<{ success: boolean; message: string }> {
	return request<{ success: boolean; message: string }>(
		`/projects/${projectSlug}/agents/${agentName}`,
		{
			method: 'DELETE'
		}
	);
}

// ============================================================================
// Messages - Inbox/Outbox
// ============================================================================

export async function getInbox(projectSlug: string, agentName: string): Promise<Message[]> {
	return request<Message[]>('/inbox', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			agent_name: agentName
		})
	});
}

export async function getOutbox(projectSlug: string, agentName: string): Promise<Message[]> {
	return request<Message[]>('/outbox', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			agent_name: agentName
		})
	});
}

export async function getMessage(id: number): Promise<Message> {
	return request<Message>(`/messages/${id}`);
}

export async function sendMessage(req: SendMessageRequest): Promise<Message> {
	return request<Message>('/message/send', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: req.project_slug,
			sender_name: req.sender_name,
			recipient_names: req.recipient_names,
			subject: req.subject,
			body_md: req.body_md,
			thread_id: req.thread_id,
			importance: req.importance ?? 'normal',
			ack_required: req.ack_required ?? false,
			cc_names: req.cc_names,
			bcc_names: req.bcc_names
		})
	});
}

export async function markMessageRead(
	projectSlug: string,
	messageId: number,
	agentName: string
): Promise<void> {
	await request<void>('/message/read', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			message_id: messageId,
			agent_name: agentName
		})
	});
}

export async function acknowledgeMessage(
	projectSlug: string,
	messageId: number,
	agentName: string
): Promise<void> {
	await request<void>('/message/acknowledge', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			message_id: messageId,
			agent_name: agentName
		})
	});
}

// ============================================================================
// Messages - Search
// ============================================================================

export async function searchMessages(req: SearchMessagesRequest): Promise<Message[]> {
	return request<Message[]>('/messages/search', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: req.project_slug,
			query: req.query,
			limit: req.limit ?? 100
		})
	});
}

// ============================================================================
// Threads
// ============================================================================

export async function getThread(projectSlug: string, threadId: string): Promise<Thread> {
	// API returns Message[] directly, wrap it in Thread format
	const messages = await request<Message[]>('/thread', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug, thread_id: threadId })
	});

	// Build Thread object from messages array
	const firstMessage = messages[0];
	return {
		thread_id: threadId,
		subject: firstMessage?.subject || '',
		messages,
		participants: [...new Set(messages.map(m => m.sender_name || ''))],
		message_count: messages.length,
		last_message_ts: messages[messages.length - 1]?.created_ts || ''
	};
}

export async function listThreads(projectSlug: string): Promise<ThreadSummary[]> {
	return request<ThreadSummary[]>('/threads', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug })
	});
}

export async function getThreadMessages(
	projectSlug: string,
	_agentName: string,
	threadId: string
): Promise<Message[]> {
	const thread = await getThread(projectSlug, threadId);
	return thread.messages;
}

// ============================================================================
// Unified Inbox (Cross-Project)
// ============================================================================

export async function fetchUnifiedInbox(limit = 1000): Promise<UnifiedInboxResponse> {
	return request<UnifiedInboxResponse>(`/unified-inbox?limit=${limit}`);
}

// ============================================================================
// File Reservations
// ============================================================================

export async function reserveFiles(req: FileReservationRequest): Promise<FileReservationGrant> {
	return request<FileReservationGrant>('/file_reservations/paths', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: req.project_slug,
			agent_name: req.agent_name,
			paths: req.paths,
			ttl_seconds: req.ttl_seconds ?? 3600,
			exclusive: req.exclusive ?? true,
			reason: req.reason
		})
	});
}

export async function listFileReservations(projectSlug: string): Promise<FileReservation[]> {
	return request<FileReservation[]>('/file_reservations/list', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug })
	});
}

export async function listAllLocks(): Promise<FileReservation[]> {
	return request<FileReservation[]>('/locks');
}

export async function releaseReservation(
	projectSlug: string,
	reservationId: number
): Promise<void> {
	await request<void>('/file_reservations/release', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug, reservation_id: reservationId })
	});
}

export async function forceReleaseReservation(
	projectSlug: string,
	reservationId: number,
	reason: string
): Promise<void> {
	await request<void>('/file_reservations/force_release', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			reservation_id: reservationId,
			reason
		})
	});
}

export async function renewReservation(
	projectSlug: string,
	reservationId: number,
	ttlSeconds: number
): Promise<FileReservation> {
	return request<FileReservation>('/file_reservations/renew', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			reservation_id: reservationId,
			ttl_seconds: ttlSeconds
		})
	});
}

// ============================================================================
// Attachments
// ============================================================================

export async function listAttachments(projectSlug?: string): Promise<Attachment[]> {
	const query = projectSlug ? `?project_slug=${encodeURIComponent(projectSlug)}` : '';
	return request<Attachment[]>(`/attachments${query}`);
}

export async function getAttachment(id: number): Promise<Blob> {
	const response = await fetch(`${API_BASE}/attachments/${id}`);
	if (!response.ok) {
		throw new Error('Failed to fetch attachment');
	}
	return response.blob();
}

export async function addAttachment(
	messageId: number,
	filename: string,
	data: Uint8Array
): Promise<Attachment> {
	return request<Attachment>('/attachments/add', {
		method: 'POST',
		body: JSON.stringify({
			message_id: messageId,
			filename,
			data: Array.from(data) // Convert to number array for JSON
		})
	});
}

// ============================================================================
// Build Slots
// ============================================================================

export async function acquireBuildSlot(
	projectSlug: string,
	agentName: string,
	slotType: string,
	ttlSeconds: number,
	reason?: string
): Promise<BuildSlot> {
	return request<BuildSlot>('/build_slots/acquire', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			agent_name: agentName,
			slot_type: slotType,
			ttl_seconds: ttlSeconds,
			reason
		})
	});
}

export async function releaseBuildSlot(projectSlug: string, slotId: number): Promise<void> {
	await request<void>('/build_slots/release', {
		method: 'POST',
		body: JSON.stringify({ project_slug: projectSlug, slot_id: slotId })
	});
}

export async function renewBuildSlot(
	projectSlug: string,
	slotId: number,
	ttlSeconds: number
): Promise<BuildSlot> {
	return request<BuildSlot>('/build_slots/renew', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			slot_id: slotId,
			ttl_seconds: ttlSeconds
		})
	});
}

// ============================================================================
// Activity
// ============================================================================

export async function listActivity(projectSlug?: string, limit = 100): Promise<ActivityItem[]> {
	const params = new URLSearchParams();
	if (projectSlug) params.set('project_slug', projectSlug);
	params.set('limit', String(limit));
	return request<ActivityItem[]>(`/activity?${params.toString()}`);
}

// ============================================================================
// Metrics
// ============================================================================

export async function listToolMetrics(): Promise<ToolMetric[]> {
	return request<ToolMetric[]>('/metrics/tools');
}

export async function getToolStats(): Promise<ToolStats> {
	return request<ToolStats>('/metrics/tools/stats');
}

// ============================================================================
// Archive
// ============================================================================

export async function listArchiveCommits(limit = 20): Promise<ArchiveCommit[]> {
	return request<ArchiveCommit[]>(`/archive/commits?limit=${limit}`);
}

export async function getArchiveCommit(sha: string): Promise<ArchiveCommit> {
	return request<ArchiveCommit>(`/archive/commits/${sha}`);
}

export async function listArchiveFiles(sha: string): Promise<ArchiveFile[]> {
	return request<ArchiveFile[]>(`/archive/files/${sha}`);
}

export async function getArchiveFileContent(sha: string, path: string): Promise<string> {
	const response = await fetch(
		`${API_BASE}/archive/file/${sha}?path=${encodeURIComponent(path)}`
	);
	if (!response.ok) {
		throw new Error('Failed to fetch archive file');
	}
	return response.text();
}

// ============================================================================
// Health
// ============================================================================

export async function checkHealth(): Promise<{ status: string }> {
	return request<{ status: string }>('/health');
}

// ============================================================================
// Dashboard Stats
// ============================================================================

export interface DashboardStats {
	projectCount: number;
	agentCount: number;
	inboxCount: number;
	messageCount: number;
	projects: Array<Project & { agentCount?: number }>;
}

/**
 * Fetch aggregated dashboard statistics.
 * This fetches projects, aggregates agent counts, and gets inbox stats.
 */
export async function getDashboardStats(): Promise<DashboardStats> {
	// Fetch projects and unified inbox in parallel
	const [projects, unifiedInbox] = await Promise.all([getProjects(), fetchUnifiedInbox(1000)]);

	// Fetch agents for each project in parallel (limit to first 10 projects for performance)
	const projectsToFetch = projects.slice(0, 10);
	const agentResults = await Promise.allSettled(
		projectsToFetch.map((p) => getAgents(p.slug).catch(() => []))
	);

	// Count total agents
	let agentCount = 0;
	const projectsWithAgents = projects.map((project, index) => {
		if (index < projectsToFetch.length) {
			const result = agentResults[index];
			const agents = result.status === 'fulfilled' ? result.value : [];
			agentCount += agents.length;
			return { ...project, agentCount: agents.length };
		}
		return { ...project, agentCount: undefined };
	});

	return {
		projectCount: projects.length,
		agentCount,
		inboxCount: unifiedInbox.messages.filter((m) => !m.is_read).length,
		messageCount: unifiedInbox.total_count,
		projects: projectsWithAgents
	};
}

// ============================================================================
// Re-export types for convenience
// ============================================================================

export type {
	Project,
	Agent,
	Message,
	Thread,
	ThreadSummary,
	FileReservation,
	FileReservationGrant,
	Attachment,
	UnifiedInboxResponse,
	ActivityItem,
	ArchiveCommit,
	ArchiveFile,
	BuildSlot,
	ToolMetric,
	ToolStats,
	ApiError,
	SendMessageRequest,
	SearchMessagesRequest,
	FileReservationRequest
};

export { ApiRequestError };
