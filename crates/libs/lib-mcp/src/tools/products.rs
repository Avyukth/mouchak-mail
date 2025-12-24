//! Product tool implementations
//!
//! Handles multi-repo coordination via products.

use lib_core::{
    ctx::Ctx,
    model::{ModelManager, message::MessageBmc, product::ProductBmc, project::ProjectBmc},
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{
    EnsureProductParams, LinkProjectToProductParams, ProductInboxParams,
    UnlinkProjectFromProductParams,
};

/// Create or get a product for multi-repo coordination.
pub async fn ensure_product_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: EnsureProductParams,
) -> Result<CallToolResult, McpError> {
    let product = ProductBmc::ensure(ctx, mm, &params.product_uid, &params.name)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let output = format!(
        "Product: {} ({})\nID: {}\nCreated: {}",
        product.name, product.product_uid, product.id, product.created_at
    );
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Link a project to a product for unified messaging.
pub async fn link_project_to_product_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: LinkProjectToProductParams,
) -> Result<CallToolResult, McpError> {
    let product = ProductBmc::get_by_uid(ctx, mm, &params.product_uid)
        .await
        .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let link_id = ProductBmc::link_project(ctx, mm, product.id, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Linked project '{}' to product '{}' (link_id: {})",
        params.project_slug, params.product_uid, link_id
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Unlink a project from a product.
pub async fn unlink_project_from_product_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: UnlinkProjectFromProductParams,
) -> Result<CallToolResult, McpError> {
    let product = ProductBmc::get_by_uid(ctx, mm, &params.product_uid)
        .await
        .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let unlinked = ProductBmc::unlink_project(ctx, mm, product.id, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = if unlinked {
        format!(
            "Unlinked project '{}' from product '{}'",
            params.project_slug, params.product_uid
        )
    } else {
        format!(
            "Project '{}' was not linked to product '{}'",
            params.project_slug, params.product_uid
        )
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List all products and their linked projects.
pub async fn list_products_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
) -> Result<CallToolResult, McpError> {
    let products = ProductBmc::list_all(ctx, mm)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!("Products ({}):\n\n", products.len());
    for p in &products {
        output.push_str(&format!(
            "- {} (uid: {}, {} projects)\n  Projects: {:?}\n",
            p.name,
            p.product_uid,
            p.project_ids.len(),
            p.project_ids
        ));
    }
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Get aggregated inbox across all projects in a product.
pub async fn product_inbox_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ProductInboxParams,
) -> Result<CallToolResult, McpError> {
    let product = ProductBmc::get_by_uid(ctx, mm, &params.product_uid)
        .await
        .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

    let project_ids = ProductBmc::get_linked_projects(ctx, mm, product.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let limit = params.limit.unwrap_or(10);
    let mut output = format!(
        "Product Inbox for '{}' ({} projects):\n\n",
        product.name,
        project_ids.len()
    );

    for project_id in project_ids {
        let project = ProjectBmc::get(ctx, mm, project_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let messages = MessageBmc::list_recent(ctx, mm, project_id, limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        output.push_str(&format!(
            "\n## Project: {} ({})\n",
            project.human_key, project.slug
        ));
        for m in &messages {
            output.push_str(&format!(
                "  - [{}] {} (from: {}, {})\n",
                m.id, m.subject, m.sender_name, m.created_ts
            ));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
