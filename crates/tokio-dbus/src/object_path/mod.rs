use self::validation::validate;
mod validation;

pub use self::object_path_error::ObjectPathError;
mod object_path_error;

pub use self::object_path::ObjectPath;
mod object_path;

#[cfg(feature = "alloc")]
pub use self::object_path_buf::ObjectPathBuf;
#[cfg(feature = "alloc")]
mod object_path_buf;

pub use self::iter::Iter;
mod iter;

#[cfg(test)]
mod tests;
