use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

use crate::feature::port::{CreateError, FeatureRepository, GetError};
use escalation::{Report, classify};
use hooq::hooq;

pub struct Db;

struct Connection {
    user_table: HashSet<usize>,
}

impl Connection {
    fn get_user(&self, id: usize) -> Option<usize> {
        self.user_table.get(&id).map(|v| *v)
    }

    #[hooq(seed)]
    fn innsert_user(&mut self, id: usize) -> Result<(), Report<&'static str>> {
        if !self.user_table.insert(id) {
            return Err("Db return false");
        }

        Ok(())
    }
}

classify! {
    &'static str, anyhow::Error => GetError::Other;
    &'static str, anyhow::Error => CreateError::Other;
}

#[hooq(seed)]
fn connect_db(flag: bool) -> Result<&'static Mutex<Connection>, Report<&'static str>> {
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

        let res = conn
            .lock()
            .map_err(|e| anyhow::Error::msg(e.to_string()))?
            .get_user(id);

        Ok(res)
    }

    fn create_user(&self, id: usize) -> Result<(), Report<CreateError>> {
        if id > 10000000000 {
            #[hooq::error = CreateError::Other]
            return Err("id > 10000000000");
        }

        #[hooq::error = CreateError::CannotConnect]
        let conn = connect_db(id != 501)?;

        let mut conn = conn.lock().map_err(|e| anyhow::Error::msg(e.to_string()))?;

        if let Some(_) = conn.get_user(id) {
            #[hooq::error = CreateError::AlreadyExist(id)]
            return Err("already exist");
        }

        conn.innsert_user(id)?;

        Ok(())
    }
}
