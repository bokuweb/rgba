pub mod arm;
pub mod thumb;
pub mod shift;

#[derive(Debug)]
pub enum PipelineStatus {
    Flush,
    Continue,
}