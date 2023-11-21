use self::validation::validate;
mod validation;

pub use self::object_path_error::ObjectPathError;
mod object_path_error;

pub use self::object_path::ObjectPath;
mod object_path;

pub use self::owned_object_path::OwnedObjectPath;
mod owned_object_path;

#[cfg(test)]
mod tests;
