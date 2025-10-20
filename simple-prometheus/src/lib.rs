pub use simple_prometheus_derive::*;

pub trait SimplePrometheus {
    fn to_prometheus_metrics(&self, server: Option<String>) -> Result<String, core::fmt::Error>;
}
