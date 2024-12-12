#[cfg(test)]
use headers::Header;
pub use range::{ByteRangeBuilder, ByteRangeSpec, Range};

mod range;

macro_rules! error_type {
    ($name:ident) => {
        #[doc(hidden)]
        pub struct $name {
            _inner: (),
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct(stringify!($name)).finish()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(stringify!($name))
            }
        }

        impl ::std::error::Error for $name {}
    };
}

pub(crate) use error_type;

#[cfg(test)]
pub(crate) fn test_decode<T: Header>(values: &[&str]) -> Option<T> {
    use headers::{HeaderMap, HeaderMapExt};
    let mut map = HeaderMap::new();
    for val in values {
        map.append(T::name(), val.parse().unwrap());
    }
    map.typed_get()
}
