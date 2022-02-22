use postgres;

pub struct DB {
    pub client: Result<postgres::Client, postgres::Error>
}

impl DB {
    pub fn new(host: &str, user: &str, password: &str, dbname: &str) -> DB {
        let cnxn_str = format!("host={} user={} password={} dbname={}",
        host, user, password, dbname);
        DB { client: postgres::Client::connect(
            cnxn_str.as_str(), postgres::NoTls)
        }
        // let test = format!("host={host} username={user} password={password} dbname={dbname}");
    }
}

// pub fn execute_query(client: DB, query: &str) -> Result<Vec<postgres::Row>, postgres::Error> {
// }