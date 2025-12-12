use e2e_tests::TestConfig;
use jugar_probar::Browser;
use std::time::Duration;

#[tokio::test]
async fn test_mobile_walkthrough_navigation() {
    // This script automates the navigation steps of the Mobile Walkthrough.
    // Due to current tooling limitations, interactive steps (clicks, typing)
    // are left for manual verification, but this script ensures all routes load.

    let mut config = TestConfig::default();
    config.web_ui_url = "http://localhost:4090".to_string(); // Default trunk serve port

    println!("Launching browser for Mobile Walkthrough...");
    let browser = Browser::launch(Default::default()).await.expect("Failed to launch browser");
    let mut page = browser.new_page().await.expect("Failed to create page");

    // 1. Dashboard & Theme
    println!("Step 1: Dashboard");
    page.goto(&config.web_ui_url).await.expect("Failed to load Dashboard");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 2. Inbox View (Compose Trigger)
    println!("Step 2: Inbox - Please verify 'Compose' button is visible");
    let inbox_url = format!("{}/inbox", config.web_ui_url);
    page.goto(&inbox_url).await.expect("Failed to load Inbox");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 3. Projects View
    println!("Step 3: Projects List");
    let projects_url = format!("{}/projects", config.web_ui_url);
    page.goto(&projects_url).await.expect("Failed to load Projects");
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("Navigation walkthrough complete. Please perform interactive steps manually.");
}
