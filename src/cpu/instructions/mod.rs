pub mod arm;
pub mod shift;
pub mod thumb;
pub mod helpers;

#[derive(Debug)]
pub enum PipelineStatus {
    Flush,
    Continue,
}

pub type ExecuteResult = (crate::types::Cycle, PipelineStatus);
