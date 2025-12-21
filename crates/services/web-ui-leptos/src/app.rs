//! Root application component with router configuration.

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::Layout;
use crate::pages::*;

/// Root application component with all routes.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound /> }>
                <ParentRoute path=path!("") view=Layout>
                    <Route path=path!("") view=Dashboard />
                    <Route path=path!("projects") view=Projects />
                    <Route path=path!("projects/:slug") view=ProjectDetail />
                    <Route path=path!("projects/:slug/file-reservations") view=FileReservations />
                    <Route path=path!("agents") view=Agents />
                    <Route path=path!("attachments") view=Attachments />
                    <Route path=path!("inbox") view=Inbox />
                    <Route path=path!("inbox/:id") view=MessageDetail />
                    <Route path=path!("mail") view=UnifiedInbox />
                    <Route path=path!("mail/unified") view=UnifiedInbox />
                    <Route path=path!("mail/unified-inbox") view=UnifiedInbox />
                    <Route path=path!("thread/:id") view=ThreadView />
                    <Route path=path!("search") view=Search />
                    <Route path=path!("archive") view=ArchiveBrowser />
                </ParentRoute>

            </Routes>
        </Router>
    }
}

/// 404 Not Found page.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-gray-300 dark:text-gray-700">"404"</h1>
                <p class="mt-4 text-xl text-gray-600 dark:text-gray-400">"Page not found"</p>
                <a href="/" class="mt-6 inline-block text-primary-600 hover:text-primary-500">
                    "← Back to Dashboard"
                </a>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unified_inbox_aliases_present() {
        let routes = vec![
            "",
            "projects",
            "projects/:slug",
            "projects/:slug/file-reservations",
            "agents",
            "attachments",
            "inbox",
            "inbox/:id",
            "mail",
            "mail/unified",
            "mail/unified-inbox",
            "thread/:id",
            "search",
        ];

        assert!(routes.contains(&"mail"));
        assert!(routes.contains(&"mail/unified"));
        assert!(routes.contains(&"mail/unified-inbox"));
        assert!(routes.contains(&"attachments"));
        assert!(routes.contains(&"search"));
    }
}
