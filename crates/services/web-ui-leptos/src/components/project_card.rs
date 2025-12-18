//! Project Card component with status badges.
//!
//! Enhanced project card matching the Python reference design with status indicators.

use leptos::prelude::*;

/// Project status enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectStatus {
    Active,
    Inactive,
}

impl ProjectStatus {
    /// Get CSS classes for the status badge
    pub fn classes(&self) -> &'static str {
        match self {
            ProjectStatus::Active => {
                "bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300"
            }
            ProjectStatus::Inactive => {
                "bg-charcoal-100 dark:bg-charcoal-700 text-charcoal-500 dark:text-charcoal-400"
            }
        }
    }

    /// Get the label for the status
    pub fn label(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "Active",
            ProjectStatus::Inactive => "Inactive",
        }
    }

    /// Get the dot color class
    pub fn dot_class(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "bg-emerald-500",
            ProjectStatus::Inactive => "bg-charcoal-400",
        }
    }
}

/// Project card component with status badges.
///
/// # Example
/// ```rust,ignore
/// view! {
///     <ProjectCard
///         slug="my-project".to_string()
///         human_key="/data/projects/my-project".to_string()
///         created_at="2025-10-26T10:30:00".to_string()
///         status=ProjectStatus::Active
///         agent_count=3
///         message_count=42
///     />
/// }
/// ```
#[component]
pub fn ProjectCard(
    /// Project slug (used for navigation)
    #[prop(into)]
    slug: String,
    /// Human-readable key (full path)
    #[prop(into)]
    human_key: String,
    /// Creation timestamp
    #[prop(into)]
    created_at: String,
    /// Project status
    #[prop(default = ProjectStatus::Active)]
    status: ProjectStatus,
    /// Number of agents
    #[prop(default = 0)]
    agent_count: usize,
    /// Number of messages
    #[prop(default = 0)]
    message_count: usize,
) -> impl IntoView {
    let href = format!("/projects/{}", slug);
    let formatted_date = format_date(&created_at);

    view! {
        <a
            href={href}
            class="card-elevated p-5 hover:shadow-lg group block transition-all hover:border-amber-300 dark:hover:border-amber-700"
        >
            // Icon
            <div class="w-12 h-12 rounded-xl bg-amber-100 dark:bg-amber-900/30 flex items-center justify-center mb-4 group-hover:scale-105 transition-transform">
                <i data-lucide="folder-open" class="icon-xl text-amber-600 dark:text-amber-400"></i>
            </div>

            // Path (truncated)
            <h3 class="font-display font-semibold text-charcoal-800 dark:text-cream-100 truncate mb-1 group-hover:text-amber-600 dark:group-hover:text-amber-400 transition-colors">
                {slug.clone()}
            </h3>
            <p class="text-sm text-charcoal-500 dark:text-charcoal-400 truncate mb-3" title={human_key.clone()}>
                {human_key.clone()}
            </p>

            // Status Badge
            <div class="flex items-center gap-2 mb-4">
                <span class={format!(
                    "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium {}",
                    status.classes()
                )}>
                    <span class={format!("w-1.5 h-1.5 rounded-full {}", status.dot_class())}></span>
                    {status.label()}
                </span>
            </div>

            // Stats Row
            <div class="flex items-center gap-4 text-sm text-charcoal-500 dark:text-charcoal-400 pt-3 border-t border-cream-200 dark:border-charcoal-700">
                <span class="flex items-center gap-1" title="Agents">
                    <i data-lucide="bot" class="icon-xs"></i>
                    {agent_count}
                </span>
                <span class="flex items-center gap-1" title="Messages">
                    <i data-lucide="mail" class="icon-xs"></i>
                    {message_count}
                </span>
                <span class="flex items-center gap-1 ml-auto" title="Created">
                    <i data-lucide="calendar" class="icon-xs"></i>
                    {formatted_date}
                </span>
            </div>
        </a>
    }
}

/// Format date string for display
fn format_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "—".to_string();
    }
    // Extract just the date part (YYYY-MM-DD)
    date_str.split('T').next().unwrap_or(date_str).to_string()
}

/// Determine project status based on activity
pub fn determine_project_status(last_active: Option<&str>, agent_count: usize) -> ProjectStatus {
    // If has agents, consider active
    if agent_count > 0 {
        return ProjectStatus::Active;
    }

    // Check last activity timestamp (simplified - would need proper date parsing)
    if let Some(ts) = last_active {
        if !ts.is_empty() {
            return ProjectStatus::Active;
        }
    }

    ProjectStatus::Inactive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_status_classes() {
        assert!(ProjectStatus::Active.classes().contains("emerald"));
        assert!(ProjectStatus::Inactive.classes().contains("charcoal"));
    }

    #[test]
    fn test_project_status_labels() {
        assert_eq!(ProjectStatus::Active.label(), "Active");
        assert_eq!(ProjectStatus::Inactive.label(), "Inactive");
    }

    #[test]
    fn test_format_date_with_time() {
        assert_eq!(format_date("2025-10-26T10:30:00"), "2025-10-26");
    }

    #[test]
    fn test_format_date_date_only() {
        assert_eq!(format_date("2025-10-26"), "2025-10-26");
    }

    #[test]
    fn test_format_date_empty() {
        assert_eq!(format_date(""), "—");
    }

    #[test]
    fn test_determine_status_with_agents() {
        assert_eq!(
            determine_project_status(None, 3),
            ProjectStatus::Active
        );
    }

    #[test]
    fn test_determine_status_no_agents_no_activity() {
        assert_eq!(
            determine_project_status(None, 0),
            ProjectStatus::Inactive
        );
    }

    #[test]
    fn test_determine_status_with_activity() {
        assert_eq!(
            determine_project_status(Some("2025-10-26"), 0),
            ProjectStatus::Active
        );
    }
}
