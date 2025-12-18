//! Project Card component with status badges.
//!
//! Enhanced project card using shadcn/ui Card and Badge components.

use crate::components::{
    Badge, BadgeVariant, Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use leptos::prelude::*;

/// Project status enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectStatus {
    Active,
    Inactive,
}

impl ProjectStatus {
    /// Convert status to BadgeVariant
    pub fn to_badge_variant(&self) -> BadgeVariant {
        match self {
            ProjectStatus::Active => BadgeVariant::Success,
            ProjectStatus::Inactive => BadgeVariant::Secondary,
        }
    }

    /// Get the label for the status
    pub fn label(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "Active",
            ProjectStatus::Inactive => "Inactive",
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
    let badge_variant = status.to_badge_variant();

    view! {
        <a href={href} class="block group h-full">
            <Card class="h-full hover:shadow-lg transition-all hover:border-amber-300 dark:hover:border-amber-700">
                <CardHeader>
                    // Icon
                    <div class="w-12 h-12 rounded-xl bg-amber-100 dark:bg-amber-900/30 flex items-center justify-center mb-2 group-hover:scale-105 transition-transform">
                        <i data-lucide="folder-open" class="icon-xl text-amber-600 dark:text-amber-400"></i>
                    </div>

                    <CardTitle class="mb-1 group-hover:text-amber-600 dark:group-hover:text-amber-400 transition-colors">
                        {slug.clone()}
                    </CardTitle>
                    <CardDescription title={human_key.clone()} class="truncate">
                        {human_key.clone()}
                    </CardDescription>
                </CardHeader>

                <CardContent>
                    <Badge variant={badge_variant}>
                        {status.label()}
                    </Badge>
                </CardContent>

                <CardFooter class="pt-0 border-t border-cream-200 dark:border-charcoal-700 mt-auto">
                   <div class="flex items-center gap-4 text-sm text-charcoal-500 dark:text-charcoal-400 w-full pt-4">
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
                </CardFooter>
            </Card>
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

    // Check last activity timestamp (simplified)
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
    fn test_project_status_mapping() {
        assert_eq!(
            ProjectStatus::Active.to_badge_variant(),
            BadgeVariant::Success
        );
        assert_eq!(
            ProjectStatus::Inactive.to_badge_variant(),
            BadgeVariant::Secondary
        );
    }

    #[test]
    fn test_project_status_labels() {
        assert_eq!(ProjectStatus::Active.label(), "Active");
        assert_eq!(ProjectStatus::Inactive.label(), "Inactive");
    }

    #[test]
    fn test_format_date() {
        assert_eq!(format_date("2025-10-26T10:30:00"), "2025-10-26");
        assert_eq!(format_date(""), "—");
    }
}
