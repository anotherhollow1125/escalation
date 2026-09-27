use escalation::{Classify, Report};

#[derive(thiserror::Error, Debug, Classify)]
#[classify(Unclassified as GetError::Other)]
pub enum GetError {
    #[error("cannnot connect to db.")]
    CannotConnect,
    #[error("something wrong")]
    Other,
}

#[derive(thiserror::Error, Debug, Classify)]
#[classify(Unclassified as CreateError::Other)]
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
