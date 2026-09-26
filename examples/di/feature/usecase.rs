use escalation::Report;

#[derive(thiserror::Error, Debug)]
pub enum UsecaseError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("already registered: {0}")]
    AlreadyRegistered(usize),
    #[error("repository return error.")]
    RepositoryError,
    #[error("other.")]
    Other,
}

pub trait Usecase {
    fn usecase(&self, s: String) -> Result<(), Report<UsecaseError>>;
}

pub mod impls {
    use super::{Usecase, UsecaseError};
    use crate::feature::port::{CreateError, FeatureRepository, GetError};
    use escalation::{Report, classify};
    use hooq::hooq;
    use std::sync::Arc;

    classify! {
        CreateError, GetError => UsecaseError::RepositoryError;
        &'static str, std::num::ParseIntError => UsecaseError::Other;
    }

    pub struct UsecaseImpl {
        pub repository: Arc<dyn FeatureRepository>,
    }

    impl Usecase for UsecaseImpl {
        #[hooq(progress)]
        fn usecase(&self, s: String) -> Result<(), Report<UsecaseError>> {
            #[hooq::error = UsecaseError::InvalidInput(s)]
            let n = s.clone().parse::<usize>()?;

            if let Some(_) = self.repository.get_user(n)? {
                #[hooq::error = UsecaseError::AlreadyRegistered(n)]
                return Err("already registered");
            }

            self.repository.create_user(n)?;

            Ok(())
        }
    }
}
