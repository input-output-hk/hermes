//! `SQLite` connection object runtime extension implementation.

mod conn;
mod host;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

#[cfg(test)]
mod tests {
  
}
