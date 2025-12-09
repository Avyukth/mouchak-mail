use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub product_uid: String,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductProjectLink {
    pub id: i64,
    pub product_id: i64,
    pub project_id: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct ProductForCreate {
    pub product_uid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductWithProjects {
    pub id: i64,
    pub product_uid: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub project_ids: Vec<i64>,
}

pub struct ProductBmc;

impl ProductBmc {
    /// Create or get a product (ensure_product)
    pub async fn ensure(_ctx: &Ctx, mm: &ModelManager, product_uid: &str, name: &str) -> Result<Product> {
        let db = mm.db();

        // Try to get existing
        let stmt = db.prepare(
            "SELECT id, product_uid, name, created_at FROM products WHERE product_uid = ?"
        ).await?;
        let mut rows = stmt.query([product_uid]).await?;

        if let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            return Ok(Product {
                id: row.get(0)?,
                product_uid: row.get(1)?,
                name: row.get(2)?,
                created_at,
            });
        }

        // Create new
        let stmt = db.prepare(
            "INSERT INTO products (product_uid, name) VALUES (?, ?) RETURNING id"
        ).await?;
        let mut rows = stmt.query((product_uid, name)).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create product".into()));
        };

        Ok(Product {
            id,
            product_uid: product_uid.to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now().naive_utc(),
        })
    }

    /// Get product by UID
    pub async fn get_by_uid(_ctx: &Ctx, mm: &ModelManager, product_uid: &str) -> Result<Product> {
        let db = mm.db();
        let stmt = db.prepare(
            "SELECT id, product_uid, name, created_at FROM products WHERE product_uid = ?"
        ).await?;
        let mut rows = stmt.query([product_uid]).await?;

        if let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            Ok(Product {
                id: row.get(0)?,
                product_uid: row.get(1)?,
                name: row.get(2)?,
                created_at,
            })
        } else {
            Err(crate::Error::ProductNotFound(product_uid.to_string()))
        }
    }

    /// List all products with their linked project IDs
    pub async fn list_all(_ctx: &Ctx, mm: &ModelManager) -> Result<Vec<ProductWithProjects>> {
        let db = mm.db();
        let stmt = db.prepare(
            "SELECT id, product_uid, name, created_at FROM products ORDER BY created_at DESC"
        ).await?;
        let mut rows = stmt.query(()).await?;

        let mut products = Vec::new();
        while let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            products.push(Product {
                id: row.get(0)?,
                product_uid: row.get(1)?,
                name: row.get(2)?,
                created_at,
            });
        }

        let mut result = Vec::new();
        for p in products {
            let project_ids = Self::get_linked_projects(_ctx, mm, p.id).await?;
            result.push(ProductWithProjects {
                id: p.id,
                product_uid: p.product_uid,
                name: p.name,
                created_at: p.created_at,
                project_ids,
            });
        }

        Ok(result)
    }

    /// Link a project to a product
    pub async fn link_project(_ctx: &Ctx, mm: &ModelManager, product_id: i64, project_id: i64) -> Result<i64> {
        let db = mm.db();
        let stmt = db.prepare(
            "INSERT OR IGNORE INTO product_project_links (product_id, project_id) VALUES (?, ?) RETURNING id"
        ).await?;
        let mut rows = stmt.query((product_id, project_id)).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            // Already exists, get the existing id
            let stmt = db.prepare(
                "SELECT id FROM product_project_links WHERE product_id = ? AND project_id = ?"
            ).await?;
            let mut rows = stmt.query((product_id, project_id)).await?;
            if let Some(row) = rows.next().await? {
                row.get::<i64>(0)?
            } else {
                0 // Should not happen
            }
        };

        Ok(id)
    }

    /// Unlink a project from a product
    pub async fn unlink_project(_ctx: &Ctx, mm: &ModelManager, product_id: i64, project_id: i64) -> Result<bool> {
        let db = mm.db();
        let stmt = db.prepare(
            "DELETE FROM product_project_links WHERE product_id = ? AND project_id = ?"
        ).await?;
        let result = stmt.execute((product_id, project_id)).await?;

        Ok(result > 0)
    }

    /// Get projects linked to a product
    pub async fn get_linked_projects(_ctx: &Ctx, mm: &ModelManager, product_id: i64) -> Result<Vec<i64>> {
        let db = mm.db();
        let stmt = db.prepare(
            "SELECT project_id FROM product_project_links WHERE product_id = ?"
        ).await?;
        let mut rows = stmt.query([product_id]).await?;

        let mut project_ids = Vec::new();
        while let Some(row) = rows.next().await? {
            project_ids.push(row.get::<i64>(0)?);
        }

        Ok(project_ids)
    }
}
