// Ports include/mln/util/exception.hpp.
//
// C++ passes errors around as `std::exception_ptr` and recovers the type with
// `catch` clauses (Map::Impl::onStyleError). The port uses
// `ExceptionPtr = Arc<dyn Error + Send + Sync>`; the exception types below
// are distinct structs, so the type can be recovered with `downcast_ref`.
// `Display` is `what()`.

use std::sync::Arc;

/// `std::exception_ptr`.
pub type ExceptionPtr = Arc<dyn std::error::Error + Send + Sync>;

macro_rules! define_exception {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(msg: impl Into<String>) -> Self {
                $name(msg.into())
            }

            /// `std::make_exception_ptr(Exception(msg))`.
            pub fn ptr(msg: impl Into<String>) -> ExceptionPtr {
                Arc::new($name(msg.into()))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::error::Error for $name {}
    };
}

define_exception!(
    /// `util::Exception` (a `std::runtime_error`).
    Exception
);
define_exception!(StyleImageException);
define_exception!(MisuseException);
define_exception!(StyleParseException);
define_exception!(StyleLoadException);
define_exception!(NotFoundException);
define_exception!(LocaleException);

define_exception!(
    /// `std::runtime_error`, for the places that throw it directly.
    RuntimeError
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downcast_recovers_the_type() {
        let e: ExceptionPtr = StyleParseException::ptr("bad");
        assert_eq!(e.to_string(), "bad");
        assert!(e.downcast_ref::<StyleParseException>().is_some());
        assert!(e.downcast_ref::<StyleLoadException>().is_none());
    }
}
