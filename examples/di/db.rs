use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use escalation::{Classify, Report, Unclassified, classify};
use hooq::hooq;

use crate::feature::port::{CreateError, FeatureRepository, GetError};

classify! {
    &'static str, DbError as GetError::Other;
    &'static str, DbError as CreateError::Other;
}

pub struct Db;

struct Connection {
    user_table: HashSet<usize>,
}

#[derive(Debug, thiserror::Error, Classify)]
#[error("Db error reason: {0}")]
struct DbError(#[classify] &'static str);

impl Connection {
    fn get_user(&self, id: usize) -> Option<usize> {
        self.user_table.get(&id).map(|v| *v)
    }

    #[hooq(escalate)]
    fn innsert_user(&mut self, id: usize) -> Result<(), Report<DbError>> {
        if !self.user_table.insert(id) {
            return Err("Db return false");
        }

        Ok(())
    }
}

#[hooq(escalate)]
fn connect_db(flag: bool) -> Result<&'static Mutex<Connection>, Report<DbError>> {
    static CONNECTION: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
        Mutex::new(Connection {
            user_table: HashSet::new(),
        })
    });

    if !flag {
        return Err("db doesn't work.");
    }

    Ok(&CONNECTION)
}

#[hooq(escalate)]
impl FeatureRepository for Db {
    fn get_user(&self, id: usize) -> Result<Option<usize>, Report<GetError>> {
        if id > 10000000000 {
            #[hooq::error = GetError::Other]
            return Err("id > 10000000000");
        }

        #[hooq::error = GetError::CannotConnect]
        let conn = connect_db(id != 500)?;

        let res = conn.lock().map_err(Unclassified::from)?.get_user(id);

        Ok(res)
    }

    fn create_user(&self, id: usize) -> Result<(), Report<CreateError>> {
        if id > 10000000000 {
            #[hooq::error = CreateError::Other]
            return Err("id > 10000000000");
        }

        #[hooq::error = CreateError::CannotConnect]
        let conn = connect_db(id != 501)?;

        let mut conn = conn.lock().map_err(Unclassified::from)?;

        if let Some(_) = conn.get_user(id) {
            #[hooq::error = CreateError::AlreadyExist(id)]
            return Err("already exist");
        }

        conn.innsert_user(id)?;

        Ok(())
    }
}
