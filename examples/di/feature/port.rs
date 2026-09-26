use escalation::Report;

#[derive(thiserror::Error, Debug)]
pub enum GetError {
    #[error("cannnot connect to db.")]
    CannotConnect,
    #[error("something wrong")]
    Other,
}

#[derive(thiserror::Error, Debug)]
pub enum CreateError {
    #[error("invalid input: {0}")]
    AlreadyExist(usize),
    #[error("cannnot connect to db.")]
    CannotConnect,
    #[error("something wrong")]
    Other,
}

pub trait FeatureRepository {
    fn get_user(&self, id: usize) -> Result<Option<usize>, Report<GetError>>;

    fn create_user(&self, id: usize) -> Result<(), Report<CreateError>>;
}
