/**
 * API Data Provider
 *
 * Implementation of DataProvider that fetches from the backend API.
 * Used in embedded mode when the SvelteKit app runs alongside the Rust server.
 *
 * This file is tree-shaken out of static builds via VITE_DATA_MODE.
 */

import type { DataProvider, DashboardStats, StaticDataMeta } from './provider';
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
	ApiError,
	FileReservation,
	Attachment
} from '$lib/api/types';

const API_BASE = '/api';
const DEFAULT_TIMEOUT = 30000;

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
// API Provider Implementation
// ============================================================================

export const apiProvider: DataProvider = {
	// ============================================================================
	// Metadata
	// ============================================================================

	async getMeta(): Promise<StaticDataMeta> {
		return {
			exportedAt: new Date().toISOString(),
			version: '1.0.0',
			mode: 'api'
		};
	},

	async checkHealth(): Promise<{ status: string }> {
		return request<{ status: string }>('/health');
	},

	// ============================================================================
	// Dashboard
	// ============================================================================

	async getDashboardStats(): Promise<DashboardStats> {
		const [projects, unifiedInbox] = await Promise.all([
			this.getProjects(),
			this.fetchUnifiedInbox(1000)
		]);

		const projectsToFetch = projects.slice(0, 10);
		const agentResults = await Promise.allSettled(
			projectsToFetch.map((p) => this.getAgents(p.slug).catch(() => []))
		);

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
	},

	// ============================================================================
	// Projects
	// ============================================================================

	async getProjects(): Promise<Project[]> {
		return request<Project[]>('/projects');
	},

	async getProjectsWithStats(): Promise<Project[]> {
		const projects = await this.getProjects();

		interface ProjectInfoResponse {
			id: number;
			slug: string;
			human_key: string;
			created_at: string;
			agent_count: number;
			message_count: number;
		}

		const statsPromises = projects.map((p) =>
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
	},

	async getProjectInfo(projectSlug: string): Promise<Project> {
		return request<Project>('/project/info', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug })
		});
	},

	// ============================================================================
	// Agents
	// ============================================================================

	async getAgents(projectSlug: string): Promise<Agent[]> {
		return request<Agent[]>(`/projects/${projectSlug}/agents`);
	},

	async getAgentProfile(projectSlug: string, agentName: string): Promise<Agent> {
		return request<Agent>('/agent/profile', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug, agent_name: agentName })
		});
	},

	// ============================================================================
	// Messages
	// ============================================================================

	async getInbox(projectSlug: string, agentName: string): Promise<Message[]> {
		return request<Message[]>('/inbox', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug, agent_name: agentName })
		});
	},

	async getOutbox(projectSlug: string, agentName: string): Promise<Message[]> {
		return request<Message[]>('/outbox', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug, agent_name: agentName })
		});
	},

	async getMessage(id: number): Promise<Message> {
		return request<Message>(`/messages/${id}`);
	},

	async searchMessages(projectSlug: string, query: string, limit = 100): Promise<Message[]> {
		return request<Message[]>('/messages/search', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug, query, limit })
		});
	},

	// ============================================================================
	// Unified Inbox
	// ============================================================================

	async fetchUnifiedInbox(limit = 1000): Promise<UnifiedInboxResponse> {
		return request<UnifiedInboxResponse>(`/unified-inbox?limit=${limit}`);
	},

	// ============================================================================
	// Threads
	// ============================================================================

	async getThread(projectSlug: string, threadId: string): Promise<Thread> {
		const messages = await request<Message[]>('/thread', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug, thread_id: threadId })
		});

		const firstMessage = messages[0];
		return {
			thread_id: threadId,
			subject: firstMessage?.subject || '',
			messages,
			participants: [...new Set(messages.map((m) => m.sender_name || ''))],
			message_count: messages.length,
			last_message_ts: messages[messages.length - 1]?.created_ts || ''
		};
	},

	async listThreads(projectSlug: string): Promise<ThreadSummary[]> {
		return request<ThreadSummary[]>('/threads', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug })
		});
	},

	// ============================================================================
	// Activity
	// ============================================================================

	async listActivity(projectSlug?: string, limit = 100): Promise<ActivityItem[]> {
		const params = new URLSearchParams();
		if (projectSlug) params.set('project_slug', projectSlug);
		params.set('limit', String(limit));
		return request<ActivityItem[]>(`/activity?${params.toString()}`);
	},

	// ============================================================================
	// Metrics
	// ============================================================================

	async listToolMetrics(): Promise<ToolMetric[]> {
		return request<ToolMetric[]>('/metrics/tools');
	},

	async getToolStats(): Promise<ToolStats> {
		return request<ToolStats>('/metrics/tools/stats');
	},

	// ============================================================================
	// Archive
	// ============================================================================

	async listArchiveCommits(limit = 20): Promise<ArchiveCommit[]> {
		return request<ArchiveCommit[]>(`/archive/commits?limit=${limit}`);
	},

	async getArchiveCommit(sha: string): Promise<ArchiveCommit> {
		return request<ArchiveCommit>(`/archive/commits/${sha}`);
	},

	async listArchiveFiles(sha: string): Promise<ArchiveFile[]> {
		return request<ArchiveFile[]>(`/archive/files/${sha}`);
	},

	async getArchiveFileContent(sha: string, path: string): Promise<string> {
		const response = await fetch(
			`${API_BASE}/archive/file/${sha}?path=${encodeURIComponent(path)}`
		);
		if (!response.ok) {
			throw new Error('Failed to fetch archive file');
		}
		return response.text();
	},

	// ============================================================================
	// File Reservations
	// ============================================================================

	async listFileReservations(projectSlug: string): Promise<FileReservation[]> {
		return request<FileReservation[]>('/file_reservations/list', {
			method: 'POST',
			body: JSON.stringify({ project_slug: projectSlug })
		});
	},

	// ============================================================================
	// Attachments
	// ============================================================================

	async listAttachments(projectSlug?: string): Promise<Attachment[]> {
		const query = projectSlug ? `?project_slug=${encodeURIComponent(projectSlug)}` : '';
		return request<Attachment[]>(`/attachments${query}`);
	},

	// ============================================================================
	// Mark Message Read
	// ============================================================================

	async markMessageRead(
		projectSlug: string,
		agentName: string,
		messageId: number
	): Promise<{ marked: boolean; message_id: number }> {
		return request<{ marked: boolean; message_id: number }>('/message/read', {
			method: 'POST',
			body: JSON.stringify({
				project_slug: projectSlug,
				agent_name: agentName,
				message_id: messageId
			})
		});
	}
};

export { ApiRequestError };
