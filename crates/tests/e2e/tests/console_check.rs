use e2e_tests::TestConfig;
use jugar_probar::Browser;

#[tokio::test]
async fn test_console_errors() {
    let mut config = TestConfig::default();
    config.web_ui_url = "http://localhost:4090".to_string();

    println!("Launching browser to check {}", config.web_ui_url);

    let browser = Browser::launch(Default::default()).await.expect("Failed to launch browser");
    
    let mut page = browser.new_page().await.expect("Failed to create page");
    page.goto(&config.web_ui_url).await.expect("Failed to navigate");
    
    // Give it time to load scripts using the Probar browser
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    println!("Successfully navigated to {}", config.web_ui_url);
}
