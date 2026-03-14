//! SSE streaming utilities.
//!
//! This module provides helpers for Server-Sent Events streaming.
//! Currently, the basic streaming is handled directly in the API handlers,
//! but this module can be extended for more advanced streaming scenarios.

// Future: Real-time streaming during execution (requires executor changes)
// For now, we capture output and stream it line by line after completion.
