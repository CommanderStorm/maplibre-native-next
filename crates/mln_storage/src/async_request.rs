// Ports include/mln/util/async_request.hpp.

/// `AsyncRequest`: dropping it cancels the request; a cancelled request's
/// callback never runs.
pub trait AsyncRequest {}
