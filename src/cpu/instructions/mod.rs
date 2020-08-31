pub mod arm;
pub mod thumb;

#[derive(Debug)]
pub enum PipelineStatus {
    Flush,
    Continue,
}