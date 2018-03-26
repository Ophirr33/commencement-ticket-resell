#![feature(conservative_impl_trait)]
#[macro_use] extern crate clap;
#[macro_use] extern crate diesel;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate config;
extern crate futures;
extern crate lettre;
extern crate lettre_email;
extern crate loggerv;
extern crate native_tls;
extern crate num_cpus;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate serde;
extern crate serde_json;

use actix::prelude::*;
use actix_web::*;
use clap::ArgMatches;
use chrono::NaiveDateTime;
use diesel::SqliteConnection;
use diesel::prelude::*;
use futures::future::{result, Future};
use rand::Rng;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug, Clone)]
enum Emailer {
    Mock,
    Gmail{ gmail_username: String, gmail_password: String }
}

impl Emailer {
    fn send_token(&self, domain: &str, username: &str, token:  i64)
        -> Result<()>
    {
        use lettre_email::EmailBuilder;
        use lettre::EmailTransport;
        use lettre::smtp::{ClientSecurity, SmtpTransportBuilder};
        use lettre::smtp::client::net::ClientTlsParameters;
        use lettre::smtp::authentication::Credentials;
        use native_tls::TlsConnector;
        let gmail_username = match self {
            &Emailer::Mock => "mock-emailer@localhost",
            &Emailer::Gmail{ ref gmail_username, .. } => gmail_username
        };
        let email = EmailBuilder::new()
            .to(format!("{}@husky.neu.edu", username))
            .from(format!("{}", gmail_username))
            .subject("Commencement Ticket Resell Confirmation")
            .text(format!("Hey {}, thanks for registering. Login with this url {}",
                          username,
                          format!("https://{}/api/confirm?username={}&token={}",
                                  domain, username, token)))
            .build()
            .map_err(DescError::from)?;
        match self {
            &Emailer::Gmail { ref gmail_password, .. } => {
                info!("Built email!");
                let connector = TlsConnector::builder().unwrap().build().unwrap();
                let security = ClientSecurity::Opportunistic(
                    ClientTlsParameters::new("smtp.gmail.com".into(), connector));
                let mut transport = SmtpTransportBuilder::new("smtp.gmail.com:587", security)
                    .map_err(DescError::from)?
                    .credentials(Credentials::new(gmail_username.to_owned(),
                    gmail_password.to_owned()))
                    .build();
                info!("Sending email!");
                transport.send(&email).map_err(DescError::from)?;
                Ok(())
            },
            &Emailer::Mock => {
                info!("Mock emailer would have sent the following email: {:?}", email);
                Ok(())
            }
        }
    }
}

// ================ CONFIGURATION =================
#[derive(Debug, Clone)]
struct Properties {
    bind_to: String,
    db: String,
    domain: String,
    emailer: Emailer,
}

impl<'a> From<ArgMatches<'a>> for Properties {
    fn from(args: ArgMatches<'a>) -> Self {
        let db = args.value_of("DB").map(|s|s.to_owned()).unwrap_or_else(|| {
            info!("No db path specified, using default value of data.db");
            "data.db".to_owned()
        });
        let port = args.value_of("PORT").unwrap_or_else(|| {
            info!("No port specified, using default value of 8080");
            "8080"
        });
        let addr = args.value_of("ADDR").unwrap_or_else(|| {
            info!("No address specified, using default value of 127.0.0.1");
            "127.0.0.1"
        });
        let domain = args.value_of("DOMAIN").map(|s|s.to_owned()).unwrap_or_else(|| {
            info!("No domain specified, using default value of localhost");
            "localhost".to_owned()
        });
        let bind_to = format!("{}:{}", addr, port);
        let emailer = match (args.value_of("USERNAME"), args.value_of("PASSWORD")) {
            (Some(u), Some(p)) => {
                Emailer::Gmail{ gmail_username: u.to_owned(), gmail_password: p.to_owned() }
            },
            _ => {
                info!("No gmailer username and password specified, using mock emailer");
                Emailer::Mock
            }
        };
        Properties { bind_to, db, domain, emailer  }
    }
}

// ================ MESSAGES =================

pub fn deserialize_number_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt<T> {
        String(String),
        Number(T),
    }

    match StringOrInt::<T>::deserialize(deserializer)? {
        StringOrInt::String(s) => s.parse::<T>().map_err(serde::de::Error::custom),
        StringOrInt::Number(i) => Ok(i),
    }
}


#[derive(Debug, Deserialize)]
struct CreateUser {
    username: String,
    buying: i32,
    selling: i32
}

impl Message for CreateUser {
    type Result = Result<bool>;
}

#[derive(Debug, Deserialize)]
struct GetUsers {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    token: i64,
    username: String
}

impl Message for GetUsers {
    type Result = Result<Vec<User>>;
}

#[derive(Debug, Deserialize, Clone)]
struct Confirm {
    username: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    token: i64
}

impl Confirm {
    fn from_params<'a>(params: &'a dev::Params<'a>) -> Result<Self> {
        let username = params.get("username")
            .map(|s| s.to_owned())
            .ok_or(DescError::new("Missing username"))?;
        let token = params.get("token")
            .map(|s| s.to_owned())
            .and_then(|t| t.parse::<i64>().ok())
            .ok_or(DescError::new("Missing token"))?;
        Ok(Confirm { username, token })
    }
}

impl Message for Confirm {
    type Result = Result<bool>;
}

#[derive(Debug, Deserialize)]
struct SetUser {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    token: i64,
    username: String,
    buying: i32,
    selling: i32
}

impl Message for SetUser {
    type Result = Result<bool>;
}

#[derive(Debug, Deserialize)]
struct DeleteUser {
    username: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    token: i64
}

impl Message for DeleteUser {
    type Result = Result<bool>;
}

// ================ DATABASE SCHEMA =================

#[derive(Debug, Serialize, Queryable)]
struct User {
    #[serde(skip_serializing)]
    access_id: i64,
    username: String,
    buying: i32,
    selling: i32,
    #[serde(skip_serializing)]
    confirmed: i32,
    created: NaiveDateTime
}

// Generated by diesel print-schema
table! {
    users (access_id, username) {
        access_id -> BigInt,
        username -> Text,
        buying -> Integer,
        selling -> Integer,
        confirmed -> Integer,
        created -> Timestamp,
    }
}

fn find_user(token: i64, uname: &str, conn: &SqliteConnection) -> Result<Option<User>> {
    use users::dsl::*;
    users.find((&token, &uname))
        .first::<User>(conn)
        .optional()
        .map_err(|e| Error::from(DescError::from(e)))
}

// ================ DATABASE HANDLER =================

struct DbHandler {
    conns: Pool<ConnectionManager<SqliteConnection>>,
    properties: Properties
}

impl Actor for DbHandler {
    type Context = SyncContext<Self>;
}

#[derive(Fail, Debug)]
#[fail(display="DE({})", cause)]
struct DescError {
    cause: String
}

impl error::ResponseError for DescError {}

impl DescError {
    fn new<T: AsRef<str>>(cause: T) -> Self {
        DescError { cause: cause.as_ref().to_owned() }
    }
}

impl<E: std::error::Error> From<E> for DescError {
    fn from(e: E) -> Self {
        info!("Building error from {}", e.description());
        DescError::new(e.description())
    }
}

impl Handler<CreateUser> for DbHandler {
    type Result = <CreateUser as Message>::Result;

    fn handle(&mut self, msg: CreateUser, _: &mut Self::Context) -> Self::Result {
        use users::dsl::*;
        use diesel::insert_into;
        if msg.username.is_empty() {
            return Err(Error::from(DescError::new("Husky username can't be empty!")));
        }
        let conn = self.conns.get().map_err(DescError::from)?;
        let conn = conn.deref();
        if let Some(_) = users.filter(username.eq(&msg.username))
            .first::<User>(conn)
            .optional()
            .map_err(DescError::from)?  {
            return Ok(true)
        };
        let mut rng = rand::thread_rng();
        let token: i64 = rng.gen();
        &self.properties.emailer.send_token(&self.properties.domain, &msg.username, token)?;
        insert_into(users)
            .values((access_id.eq(token),
                     username.eq(&msg.username),
                     buying.eq(msg.buying),
                     selling.eq(msg.selling),
                     confirmed.eq(0)))
            .execute(conn)
            .map_err(DescError::from)?;
        Ok(true)
    }
}

impl Handler<GetUsers> for DbHandler {
    type Result = <GetUsers as Message>::Result;

    fn handle(&mut self, msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        use users::dsl::*;
        let conn = self.conns.get().map_err(DescError::from)?;
        let conn = conn.deref();
        if let None = find_user(msg.token, &msg.username, conn)? {
            return Err(Error::from(error::ErrorUnauthorized("Invalid token")))
        }
        let u = users.filter(confirmed.eq(1))
                     .order(created.asc())
                     .load::<User>(conn)
                     .map_err(DescError::from)?;
        Ok(u)
    }
}

impl Handler<Confirm> for DbHandler {
    type Result = <Confirm as Message>::Result;

    fn handle(&mut self, msg: Confirm, _: &mut Self::Context) -> Self::Result {
        use users::dsl::*;
        let conn = self.conns.get().map_err(DescError::from)?;
        let conn = conn.deref();
        diesel::update(users.find((&msg.token, &msg.username)))
            .set(confirmed.eq(1))
            .execute(conn)
            .map(|c| c == 1)
            .map_err(|e| Error::from(DescError::from(e)))
    }
}

impl Handler<SetUser> for DbHandler {
    type Result = <SetUser as Message>::Result;

    fn handle(&mut self, msg: SetUser, _: &mut Self::Context) -> Self::Result {
        use users::dsl::*;
        let conn = self.conns.get().map_err(DescError::from)?;
        let conn = conn.deref();
        diesel::update(users.filter(access_id.eq(&msg.token)
                                    .and(username.eq(&msg.username))))
            .set((buying.eq(&msg.buying), selling.eq(&msg.selling)))
            .execute(conn)
            .map(|c| c == 1)
            .map_err(|e| Error::from(DescError::from(e)))
    }
}

impl Handler<DeleteUser> for DbHandler {
    type Result = <DeleteUser as Message>::Result;

    fn handle(&mut self, msg: DeleteUser, _: &mut Self::Context) -> Self::Result {
        use users::dsl::*;
        let conn = self.conns.get().map_err(DescError::from)?;
        let conn = conn.deref();
        diesel::delete(users.find((&msg.token, &msg.username)))
            .execute(conn)
            .map(|c| c == 1)
            .map_err(|e| Error::from(DescError::from(e)))
    }
}

// ================ ROUTING =================

trait AsResult<T: Serialize> {
    fn as_result(self) -> Result<T>;
}

impl<T: Serialize> AsResult<T> for Result<T> {
    fn as_result(self) -> Result<T> {
        self
    }
}

// A generic handler that deserializes an incoming json value, passes it to the backend service,
// and serializes the result back into json and returns HTTP OK
fn generic_req<'a, T, R>(req: HttpRequest<State>) -> impl Future<Item=HttpResponse, Error=Error>
    where for<'de> T: Deserialize<'de>,
          T: Message + Send,
          R: Serialize,
          <T as Message>::Result: Send+AsResult<R>,
          DbHandler: Handler<T>
{
    let addr = req.state().addr.clone();
    req.json().from_err().and_then(move |body: T| {
        addr.send(body)
            .from_err()
            .and_then(|res: <T as Message>::Result| {
                match res.as_result() {
                    Ok(g) => httpcodes::HTTPOk.build().json(g),
                    Err(e) => Ok(e.cause().error_response())
                }
            })
    })
}

fn confirm_req(req: HttpRequest<State>) -> impl Future<Item=HttpResponse, Error=Error> {
    let addr = req.state().addr.clone();
    result(Confirm::from_params(req.query()))
        .and_then(move |confirm| {
            let cookie = format!("tokenuser:{}^{}; Path=/",
                                 confirm.username, confirm.token);
            addr.send(confirm)
               .from_err()
               .and_then(move |res: Result<bool>| {
                   match res {
                       Err(e) => Ok(e.cause().error_response()),
                       Ok(b) => {
                           let cookie = if b {
                               cookie
                           } else {
                               "".into()
                           };
                           let resp = httpcodes::HTTPFound.build()
                               .header("Set-Cookie", cookie)
                               .header("Location", "/index.html")
                               .finish()?;
                           Ok(resp)
                       },
                   }
               })
        })
}


// ================ SETUP =================

struct State {
    addr: Addr<Syn, DbHandler>
}

fn make_app(addr: &Addr<Syn, DbHandler>) -> Application<State> {
    Application::with_state(State{addr: addr.clone()})
        .middleware(middleware::Logger::default())
        .middleware(middleware::DefaultHeaders::build()
                    .header("Referrer-Policy", "no-referrer")
                    .header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
                    .header("Vary", "Upgrade-Insecure-Requests")
                    .header("X-Frame-Options", "Deny")
                    .finish())
        .resource("/api/sign-up", |r| {
            r.method(Method::POST).a(generic_req::<CreateUser, bool>)
        })
        .resource("/api/get-users", |r| {
            r.method(Method::POST).a(generic_req::<GetUsers, Vec<User>>)
        })
        .resource("/api/set-user", |r| {
            r.method(Method::POST).a(generic_req::<SetUser, bool>)
        })
        .resource("/api/delete-user", |r| {
            r.method(Method::POST).a(generic_req::<DeleteUser, bool>)
        })
        .resource("/api/confirm", |r| {
            r.method(Method::GET).a(confirm_req)
        })
}

fn main() {
    loggerv::init_with_level(log::Level::Info).unwrap();
    let sys = System::new("commencement-tickets");
    let properties: Properties = clap_app!(commencement_tickets =>
        (version: "1.0")
        (author: "Ty Coghlan <coghlan.ty@gmail.com>")
        (about: "Simple web server for NEU Commencement Ticket Resell")
        (@arg DB: --db +takes_value "Sets the sqlite3 database path")
        (@arg PORT: -p --port +takes_value "Sets the port for the webserver")
        (@arg ADDR: --addr +takes_value "Sets the address the webserver will bind to")
        (@arg DOMAIN: --domain +takes_value "Sets the dns domain used in email confirmations")
        (@arg USERNAME: --username +takes_value
                              "Sets the gmail username used for sending confirmations")
        (@arg PASSWORD: --password +takes_value
                              "Sets the gmail password used for sending confirmations")
        ).get_matches().into();
    let pclone = properties.clone();
    info!("Using properties: {:?}", properties);
    let manager = ConnectionManager::<SqliteConnection>::new(properties.db);
    let conns = Pool::builder()
        .build(manager)
        .expect("Failed to init db connection pool");

    let addr = SyncArbiter::start(num_cpus::get(), move || {
        DbHandler{ conns: conns.clone(), properties: pclone.clone() }
    });

    HttpServer::new(move || make_app(&addr))
        .bind(properties.bind_to).unwrap()
        .start();
    let _ = sys.run();
}
