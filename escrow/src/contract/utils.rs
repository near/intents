use core::convert::Infallible;

// TODO: rename
pub fn single<T>(v: Vec<T>) -> Option<T> {
    let [a] = v.try_into().ok()?;
    Some(a)
}

pub trait ResultExt {
    type Ok;

    fn into_ok(self) -> Self::Ok;
}

impl<T> ResultExt for Result<T, Infallible> {
    type Ok = T;

    fn into_ok(self) -> T {
        match self {
            Ok(v) => v,
        }
    }
}
