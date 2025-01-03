pub type AppResult<T> = std::result::Result<T, AppError>;
pub type AppError = Box<dyn std::error::Error>;
