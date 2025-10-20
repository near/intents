pub trait NearSdkLog {
    fn to_near_sdk_log(&self) -> String;
}
