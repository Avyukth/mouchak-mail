//! Request context for authentication and authorization.
//!
//! The [`Ctx`] struct provides request-scoped context for identifying
//! the user making a request. This is used for audit logging and
//! future RBAC (Role-Based Access Control) implementation.

/// Request context containing user identification.
///
/// `Ctx` is passed to all BMC methods to identify the user making
/// the request. Currently used for audit logging, with future plans
/// for role-based access control.
///
/// # Examples
///
/// ```
/// use lib_core::ctx::Ctx;
///
/// // Create a root context for system operations
/// let ctx = Ctx::root_ctx();
/// assert_eq!(ctx.user_id(), 0);
///
/// // Create a context for a specific user
/// let user_ctx = Ctx::new(42);
/// assert_eq!(user_ctx.user_id(), 42);
/// ```
#[derive(Clone, Debug)]
pub struct Ctx {
    user_id: i64,
}

impl Ctx {
    /// Creates a root context for system-level operations.
    ///
    /// The root context has `user_id = 0` and is used for
    /// background tasks, migrations, and system operations
    /// that aren't associated with a specific user.
    ///
    /// # Examples
    ///
    /// ```
    /// use lib_core::ctx::Ctx;
    ///
    /// let ctx = Ctx::root_ctx();
    /// assert_eq!(ctx.user_id(), 0);
    /// ```
    pub fn root_ctx() -> Self {
        Ctx { user_id: 0 }
    }

    /// Creates a new context for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The database ID of the authenticated user
    ///
    /// # Examples
    ///
    /// ```
    /// use lib_core::ctx::Ctx;
    ///
    /// let ctx = Ctx::new(123);
    /// assert_eq!(ctx.user_id(), 123);
    /// ```
    pub fn new(user_id: i64) -> Self {
        Ctx { user_id }
    }

    /// Returns the user ID associated with this context.
    ///
    /// # Returns
    ///
    /// The user's database ID, or 0 for root context.
    pub fn user_id(&self) -> i64 {
        self.user_id
    }
}
