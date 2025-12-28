/**
 * Static Data Provider
 *
 * Implementation of DataProvider that reads from pre-bundled JSON files.
 * Used for GitHub Pages deployment where no backend API is available.
 *
 * This file is tree-shaken out of embedded builds via VITE_DATA_MODE.
 *
 * Expected static data structure in /data/:
 * - meta.json: { exportedAt, version }
 * - projects.json: Project[]
 * - agents.json: { [projectSlug]: Agent[] }
 * - messages.json: Message[]
 * - threads.json: { [projectSlug]: ThreadSummary[] }
 * - dashboard.json: DashboardStats
 * - activity.json: ActivityItem[]
 * - archive.json: { commits: ArchiveCommit[], files: { [sha]: ArchiveFile[] } }
 */

import type { DataProvider, DashboardStats, StaticDataMeta } from './provider';
import type {
	Project,
	Agent,
	Message,
	Thread,
	ThreadSummary,
	UnifiedInboxResponse,
	UnifiedInboxMessage,
	ActivityItem,
	ArchiveCommit,
	ArchiveFile,
	ToolMetric,
	ToolStats,
	FileReservation,
	Attachment
} from '$lib/api/types';

// ============================================================================
// Static Data Cache
// ============================================================================

interface StaticDataCache {
	meta?: StaticDataMeta;
	projects?: Project[];
	agents?: Record<string, Agent[]>;
	messages?: Message[];
	threads?: Record<string, ThreadSummary[]>;
	dashboard?: DashboardStats;
	activity?: ActivityItem[];
	archive?: {
		commits: ArchiveCommit[];
		files: Record<string, ArchiveFile[]>;
	};
}

const cache: StaticDataCache = {};

// ============================================================================
// Data Loading
// ============================================================================

async function loadJson<T>(path: string): Promise<T> {
	const basePath = import.meta.env.BASE_URL || '/';
	const url = `${basePath}data/${path}`;

	try {
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error(`Failed to load ${path}: ${response.statusText}`);
		}
		return response.json();
	} catch (error) {
		console.warn(`[StaticProvider] Could not load ${path}:`, error);
		throw error;
	}
}

async function ensureMeta(): Promise<StaticDataMeta> {
	if (!cache.meta) {
		try {
			cache.meta = await loadJson<StaticDataMeta>('meta.json');
		} catch {
			cache.meta = {
				exportedAt: 'unknown',
				version: '1.0.0',
				mode: 'static'
			};
		}
	}
	return cache.meta;
}

async function ensureProjects(): Promise<Project[]> {
	if (!cache.projects) {
		try {
			cache.projects = await loadJson<Project[]>('projects.json');
		} catch {
			cache.projects = [];
		}
	}
	return cache.projects;
}

async function ensureAgents(): Promise<Record<string, Agent[]>> {
	if (!cache.agents) {
		try {
			cache.agents = await loadJson<Record<string, Agent[]>>('agents.json');
		} catch {
			cache.agents = {};
		}
	}
	return cache.agents;
}

async function ensureMessages(): Promise<Message[]> {
	if (!cache.messages) {
		try {
			cache.messages = await loadJson<Message[]>('messages.json');
		} catch {
			cache.messages = [];
		}
	}
	return cache.messages;
}

async function ensureThreads(): Promise<Record<string, ThreadSummary[]>> {
	if (!cache.threads) {
		try {
			cache.threads = await loadJson<Record<string, ThreadSummary[]>>('threads.json');
		} catch {
			cache.threads = {};
		}
	}
	return cache.threads;
}

async function ensureDashboard(): Promise<DashboardStats> {
	if (!cache.dashboard) {
		try {
			cache.dashboard = await loadJson<DashboardStats>('dashboard.json');
		} catch {
			// Build from other data if dashboard.json doesn't exist
			const projects = await ensureProjects();
			const messages = await ensureMessages();
			const agents = await ensureAgents();

			let agentCount = 0;
			Object.values(agents).forEach((arr) => {
				agentCount += arr.length;
			});

			cache.dashboard = {
				projectCount: projects.length,
				agentCount,
				inboxCount: messages.filter((m) => !m.is_read).length,
				messageCount: messages.length,
				projects: projects.map((p) => ({
					...p,
					agentCount: agents[p.slug]?.length ?? 0
				}))
			};
		}
	}
	return cache.dashboard;
}

async function ensureActivity(): Promise<ActivityItem[]> {
	if (!cache.activity) {
		try {
			cache.activity = await loadJson<ActivityItem[]>('activity.json');
		} catch {
			cache.activity = [];
		}
	}
	return cache.activity;
}

async function ensureArchive(): Promise<{
	commits: ArchiveCommit[];
	files: Record<string, ArchiveFile[]>;
}> {
	if (!cache.archive) {
		try {
			cache.archive = await loadJson<{
				commits: ArchiveCommit[];
				files: Record<string, ArchiveFile[]>;
			}>('archive.json');
		} catch {
			cache.archive = { commits: [], files: {} };
		}
	}
	return cache.archive;
}

// ============================================================================
// Helper Functions
// ============================================================================

function createRelativeTime(dateStr: string): string {
	const date = new Date(dateStr);
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const diffMins = Math.floor(diffMs / 60000);
	const diffHours = Math.floor(diffMins / 60);
	const diffDays = Math.floor(diffHours / 24);

	if (diffMins < 1) return 'just now';
	if (diffMins < 60) return `${diffMins}m ago`;
	if (diffHours < 24) return `${diffHours}h ago`;
	if (diffDays < 7) return `${diffDays}d ago`;
	return date.toLocaleDateString();
}

function createExcerpt(body: string, maxLength = 100): string {
	const cleaned = body.replace(/[#*_`\[\]]/g, '').trim();
	if (cleaned.length <= maxLength) return cleaned;
	return cleaned.substring(0, maxLength - 3) + '...';
}

// ============================================================================
// Static Provider Implementation
// ============================================================================

export const staticProvider: DataProvider = {
	// ============================================================================
	// Metadata
	// ============================================================================

	async getMeta(): Promise<StaticDataMeta> {
		return ensureMeta();
	},

	async checkHealth(): Promise<{ status: string }> {
		// Always return 'demo' status in static mode
		return { status: 'demo' };
	},

	// ============================================================================
	// Dashboard
	// ============================================================================

	async getDashboardStats(): Promise<DashboardStats> {
		return ensureDashboard();
	},

	// ============================================================================
	// Projects
	// ============================================================================

	async getProjects(): Promise<Project[]> {
		return ensureProjects();
	},

	async getProjectsWithStats(): Promise<Project[]> {
		const projects = await ensureProjects();
		const agents = await ensureAgents();
		const messages = await ensureMessages();

		return projects.map((p) => {
			const projectAgents = agents[p.slug] ?? [];
			const projectMessages = messages.filter((m) => m.project_slug === p.slug);

			return {
				...p,
				agent_count: projectAgents.length,
				message_count: projectMessages.length
			};
		});
	},

	async getProjectInfo(projectSlug: string): Promise<Project> {
		const projects = await ensureProjects();
		const project = projects.find((p) => p.slug === projectSlug);
		if (!project) {
			throw new Error(`Project not found: ${projectSlug}`);
		}
		return project;
	},

	// ============================================================================
	// Agents
	// ============================================================================

	async getAgents(projectSlug: string): Promise<Agent[]> {
		const agents = await ensureAgents();
		return agents[projectSlug] ?? [];
	},

	async getAgentProfile(projectSlug: string, agentName: string): Promise<Agent> {
		const agents = await this.getAgents(projectSlug);
		const agent = agents.find((a) => a.name === agentName);
		if (!agent) {
			throw new Error(`Agent not found: ${agentName} in ${projectSlug}`);
		}
		return agent;
	},

	// ============================================================================
	// Messages
	// ============================================================================

	async getInbox(projectSlug: string, agentName: string): Promise<Message[]> {
		const messages = await ensureMessages();
		return messages.filter(
			(m) =>
				m.project_slug === projectSlug &&
				(m.recipient_names?.includes(agentName) || m.recipients?.includes(agentName))
		);
	},

	async getOutbox(projectSlug: string, agentName: string): Promise<Message[]> {
		const messages = await ensureMessages();
		return messages.filter((m) => m.project_slug === projectSlug && m.sender_name === agentName);
	},

	async getMessage(id: number): Promise<Message> {
		const messages = await ensureMessages();
		const message = messages.find((m) => m.id === id);
		if (!message) {
			throw new Error(`Message not found: ${id}`);
		}
		return message;
	},

	async searchMessages(projectSlug: string, query: string, limit = 100): Promise<Message[]> {
		const messages = await ensureMessages();
		const lowerQuery = query.toLowerCase();

		return messages
			.filter(
				(m) =>
					m.project_slug === projectSlug &&
					(m.subject.toLowerCase().includes(lowerQuery) ||
						m.body_md.toLowerCase().includes(lowerQuery) ||
						m.sender_name?.toLowerCase().includes(lowerQuery))
			)
			.slice(0, limit);
	},

	// ============================================================================
	// Unified Inbox
	// ============================================================================

	async fetchUnifiedInbox(limit = 1000): Promise<UnifiedInboxResponse> {
		const messages = await ensureMessages();

		// Sort by created_ts descending and limit
		const sortedMessages = [...messages]
			.sort((a, b) => new Date(b.created_ts).getTime() - new Date(a.created_ts).getTime())
			.slice(0, limit);

		// Enrich with unified inbox fields
		const enrichedMessages: UnifiedInboxMessage[] = sortedMessages.map((m) => ({
			...m,
			project_slug: m.project_slug || '',
			sender_name: m.sender_name || 'Unknown',
			recipient_names: m.recipient_names || m.recipients || [],
			created_relative: createRelativeTime(m.created_ts),
			excerpt: m.excerpt || createExcerpt(m.body_md)
		}));

		return {
			messages: enrichedMessages,
			total_count: messages.length
		};
	},

	// ============================================================================
	// Threads
	// ============================================================================

	async getThread(projectSlug: string, threadId: string): Promise<Thread> {
		const messages = await ensureMessages();
		const threadMessages = messages.filter(
			(m) => m.project_slug === projectSlug && m.thread_id === threadId
		);

		if (threadMessages.length === 0) {
			throw new Error(`Thread not found: ${threadId}`);
		}

		// Sort by created_ts ascending
		threadMessages.sort(
			(a, b) => new Date(a.created_ts).getTime() - new Date(b.created_ts).getTime()
		);

		const firstMessage = threadMessages[0];
		return {
			thread_id: threadId,
			subject: firstMessage.subject,
			messages: threadMessages,
			participants: [...new Set(threadMessages.map((m) => m.sender_name || ''))],
			message_count: threadMessages.length,
			last_message_ts: threadMessages[threadMessages.length - 1].created_ts
		};
	},

	async listThreads(projectSlug: string): Promise<ThreadSummary[]> {
		const threads = await ensureThreads();
		return threads[projectSlug] ?? [];
	},

	// ============================================================================
	// Activity
	// ============================================================================

	async listActivity(projectSlug?: string, limit = 100): Promise<ActivityItem[]> {
		const activity = await ensureActivity();

		let filtered = activity;
		if (projectSlug) {
			const projects = await ensureProjects();
			const project = projects.find((p) => p.slug === projectSlug);
			if (project) {
				filtered = activity.filter((a) => a.project_id === project.id);
			}
		}

		return filtered.slice(0, limit);
	},

	// ============================================================================
	// Metrics (not available in static mode)
	// ============================================================================

	async listToolMetrics(): Promise<ToolMetric[]> {
		// Metrics are not available in static mode
		return [];
	},

	async getToolStats(): Promise<ToolStats> {
		// Stats are not available in static mode
		return {
			total_calls: 0,
			total_errors: 0,
			avg_duration_ms: 0,
			top_tools: []
		};
	},

	// ============================================================================
	// Archive
	// ============================================================================

	async listArchiveCommits(limit = 20): Promise<ArchiveCommit[]> {
		const archive = await ensureArchive();
		return archive.commits.slice(0, limit);
	},

	async getArchiveCommit(sha: string): Promise<ArchiveCommit> {
		const archive = await ensureArchive();
		const commit = archive.commits.find((c) => c.sha === sha);
		if (!commit) {
			throw new Error(`Archive commit not found: ${sha}`);
		}
		return commit;
	},

	async listArchiveFiles(sha: string): Promise<ArchiveFile[]> {
		const archive = await ensureArchive();
		return archive.files[sha] ?? [];
	},

	async getArchiveFileContent(_sha: string, _path: string): Promise<string> {
		// File content is not available in static mode
		return '# Archive file content not available in demo mode\n\nThis is a static archive export.';
	},

	// ============================================================================
	// File Reservations (not available in static mode)
	// ============================================================================

	async listFileReservations(_projectSlug: string): Promise<FileReservation[]> {
		// File reservations are not available in static mode
		return [];
	},

	// ============================================================================
	// Attachments (not available in static mode)
	// ============================================================================

	async listAttachments(_projectSlug?: string): Promise<Attachment[]> {
		// Attachments are not available in static mode
		return [];
	},

	// ============================================================================
	// Message Actions (not available in static mode)
	// ============================================================================

	async markMessageRead(
		_projectSlug: string,
		_agentName: string,
		messageId: number
	): Promise<{ marked: boolean; message_id: number }> {
		// No-op in static mode - just return success without backend sync
		return { marked: true, message_id: messageId };
	}
};
