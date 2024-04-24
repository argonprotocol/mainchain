/// Return early with an error.
///
/// This macro is equivalent to `return Err(`[`anyhow!($args...)`][anyhow!]`)`.
///
/// The surrounding function's or closure's return value is required to be
/// `Result<_,`[`anyhow::Error`][crate::Error]`>`.
///
/// [anyhow!]: crate::anyhow
///
/// # Example
///
/// ```
/// # use anyhow::{bail, Result};
/// #
/// # fn has_permission(user: usize, resource: usize) -> bool {
/// #     true
/// # }
/// #
/// # fn main() -> Result<()> {
/// #     let user = 0;
/// #     let resource = 0;
/// #
/// if !has_permission(user, resource) {
///     bail!("permission denied for accessing {}", resource);
/// }
/// #     Ok(())
/// # }
/// ```
///
/// ```
/// # use anyhow::{bail, Result};
/// # use thiserror::Error;
/// #
/// # const MAX_DEPTH: usize = 1;
/// #
/// #[derive(Error, Debug)]
/// enum ScienceError {
///     #[error("recursion limit exceeded")]
///     RecursionLimitExceeded,
///     # #[error("...")]
///     # More = (stringify! {
///     ...
///     # }, 1).1,
/// }
///
/// # fn main() -> Result<()> {
/// #     let depth = 0;
/// #
/// if depth > MAX_DEPTH {
///     bail!(ScienceError::RecursionLimitExceeded);
/// }
/// #     Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::anyhow!($msg).into())
    };
    ($err:expr $(,)?) => {
        return Err($crate::anyhow!($err).into())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::anyhow!($fmt, $($arg)*).into())
    };
}
