
#[cfg(test)]
mod tests {
    use lib_core::ctx::Ctx;
    use lib_core::model::ModelManager;
    use lib_core::model::project::ProjectBmc;
    use lib_core::model::agent::{AgentBmc, AgentForCreate};
    use lib_core::model::message::{MessageBmc, MessageForCreate};
    use lib_core::model::tool_metric::{ToolMetricBmc, ToolMetricForCreate};
    use lib_core::model::activity::ActivityBmc;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
        use libsql::Builder;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let archive_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();

        let db = Builder::new_local(&db_path).build().await.unwrap();
        let conn = db.connect().unwrap();

        let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;
        // Apply all schemas
        let schema1 = include_str!("../../../../migrations/001_initial_schema.sql");
        conn.execute_batch(schema1).await.unwrap();
        let schema2 = include_str!("../../../../migrations/002_agent_capabilities.sql");
        conn.execute_batch(schema2).await.unwrap();
        let schema3 = include_str!("../../../../migrations/003_tool_metrics.sql");
        conn.execute_batch(schema3).await.unwrap();
        let schema4 = include_str!("../../../../migrations/004_attachments.sql");
        conn.execute_batch(schema4).await.unwrap();

        let mm = ModelManager::new_for_test(conn, archive_root);
        (Arc::new(mm), temp_dir)
    }

    #[tokio::test]
    async fn test_recent_activity_aggregation() {
        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // 1. Create Project
        let pid = ProjectBmc::create(&ctx, &mm, "act-test", "Activity Test").await.unwrap();

        // 2. Create Agent (Activity 1)
        // Sleep slightly to ensure distinct timestamps if ms precision
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let agent_c = AgentForCreate {
            project_id: pid,
            name: "ActAgent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        };
        let aid = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        // 3. Send Message (Activity 2)
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let msg_c = MessageForCreate {
            project_id: pid,
            sender_id: aid,
            recipient_ids: vec![aid],
            cc_ids: None,
            bcc_ids: None,
            subject: "Act Msg".into(),
            body_md: "Body".into(),
            thread_id: None,
            importance: None,
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

        // 4. Record Tool Metric (Activity 3)
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let metric = ToolMetricForCreate {
            project_id: Some(pid),
            agent_id: Some(aid),
            tool_name: "test_tool".into(),
            args_json: Some("{}".into()),
            status: "success".into(),
            error_code: None,
            duration_ms: 50,
        };
        ToolMetricBmc::create(&ctx, &mm, metric).await.unwrap();

        // 5. Query Activity
        let activities = ActivityBmc::list_recent(&ctx, &mm, pid, 10).await.unwrap();

        // Verify count
        assert_eq!(activities.len(), 3);
        
        // Verify Sort Order (Newest first)
        assert_eq!(activities[0].kind, "tool");
        assert_eq!(activities[1].kind, "message");
        assert_eq!(activities[2].kind, "agent");

        // Verify content
        assert_eq!(activities[0].title, "Tool Used: test_tool");
        assert_eq!(activities[1].title, "Act Msg");
        assert!(activities[2].title.contains("Agent Created"));
    }
}
