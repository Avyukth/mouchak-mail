/**
 * API Client for MCP Agent Mail backend
 * Backend runs on http://localhost:8000
 */

const API_BASE = '/api';

export interface Project {
	id: number;
	slug: string;
	human_key: string;
	created_at: string;
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
}

export interface Message {
	id: number;
	project_id: number;
	sender_id: number;
	thread_id: string | null;
	subject: string;
	body_md: string;
	importance: string;
	ack_required: boolean;
	created_ts: string;
}

export interface ApiError {
	error: string;
	message: string;
}

async function request<T>(
	endpoint: string,
	options: RequestInit = {}
): Promise<T> {
	const url = `${API_BASE}${endpoint}`;
	const response = await fetch(url, {
		headers: {
			'Content-Type': 'application/json',
			...options.headers
		},
		...options
	});

	if (!response.ok) {
		const error: ApiError = await response.json().catch(() => ({
			error: 'unknown',
			message: response.statusText
		}));
		throw new Error(error.message || 'Request failed');
	}

	return response.json();
}

// Projects
export async function getProjects(): Promise<Project[]> {
	return request<Project[]>('/projects');
}

export async function ensureProject(humanKey: string): Promise<Project> {
	return request<Project>('/project/ensure', {
		method: 'POST',
		body: JSON.stringify({ human_key: humanKey })
	});
}

// Agents
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

// Messages
export async function getInbox(
	projectSlug: string,
	agentName: string
): Promise<Message[]> {
	return request<Message[]>('/inbox', {
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

export async function sendMessage(
	projectSlug: string,
	senderName: string,
	recipientNames: string[],
	subject: string,
	bodyMd: string,
	threadId?: string,
	importance: string = 'normal',
	ackRequired: boolean = false
): Promise<Message> {
	return request<Message>('/message/send', {
		method: 'POST',
		body: JSON.stringify({
			project_slug: projectSlug,
			sender_name: senderName,
			recipient_names: recipientNames,
			subject,
			body_md: bodyMd,
			thread_id: threadId,
			importance,
			ack_required: ackRequired
		})
	});
}

// Health
export async function checkHealth(): Promise<{ status: string }> {
	return request<{ status: string }>('/health');
}
