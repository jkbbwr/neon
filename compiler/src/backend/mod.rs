//! Backends: the seam between the backend-independent IR and a concrete target. C is
//! the only implementation. See `docs/design/ir.md`.
//!
//! A backend receives an IR `Program` that is already fully lowered — monomorphised,
//! refcounted, every repr concrete — and is not permitted to make semantic decisions
//! about it. Anything it cannot express is a bug upstream, which is why `ctype` panics
//! on a repr it cannot pin rather than picking something that compiles.
//!
//! `ctype` is private on purpose: C struct names, witness names and the mangling scheme
//! are the C backend's internal vocabulary, and nothing outside `backend` should be able
//! to depend on the shape of the generated source.

pub mod c;
mod ctype;
