// Ports include/mln/util/enum.hpp: `Enum<T>::toString` / `Enum<T>::toEnum`
// and the `MBGL_DEFINE_ENUM` macro (here `define_enum!`). The module is
// `enum_` because `enum` is a Rust keyword.

/// `mln::Enum<T>`: the string names of an enum, as defined with
/// `MBGL_DEFINE_ENUM`.
pub trait Enum: Sized + Copy + 'static {
    /// `Enum<T>::toString`. C++ asserts that the value has a name; enums
    /// defined with `define_enum!` panic the same way in debug builds.
    fn to_string(self) -> &'static str;

    /// `Enum<T>::toEnum`: the first value whose name equals `s`.
    fn to_enum(s: &str) -> Option<Self>;
}

/// `MBGL_DEFINE_ENUM(T, {{T::A, "a"}, ...})`: implements [`Enum`] for `T`
/// by linear search through the name table, in table order (as the C++
/// `std::find_if` does).
#[macro_export]
macro_rules! define_enum {
    ($t:ty, [$(($value:expr, $name:expr)),* $(,)?]) => {
        impl $crate::enum_::Enum for $t {
            fn to_string(self) -> &'static str {
                const NAMES: &[($t, &str)] = &[$(($value, $name)),*];
                match NAMES.iter().find(|v| v.0 == self) {
                    Some(v) => v.1,
                    None => {
                        debug_assert!(false, "enum value without a name");
                        ""
                    }
                }
            }

            fn to_enum(s: &str) -> Option<Self> {
                const NAMES: &[($t, &str)] = &[$(($value, $name)),*];
                NAMES.iter().find(|v| v.1 == s).map(|v| v.0)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::Enum;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Sample {
        A,
        B,
    }

    crate::define_enum!(Sample, [(Sample::A, "a"), (Sample::B, "b")]);

    #[test]
    fn round_trip() {
        assert_eq!(Sample::A.to_string(), "a");
        assert_eq!(Sample::to_enum("b"), Some(Sample::B));
        assert_eq!(Sample::to_enum("c"), None);
    }
}
