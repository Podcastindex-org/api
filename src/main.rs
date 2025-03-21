use poem::{listener::TcpListener, EndpointExt, Route, web::Data, Request};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService, SecurityScheme};
use poem_openapi::auth::{ApiKey};
use sqlx::{Pool, MySql, Error, FromRow};
use sqlx::mysql::{MySqlPool};
use std::{sync::Mutex, collections::HashMap};
use once_cell::sync::Lazy;
use dotenv::dotenv;
use std::env;
use std::io::ErrorKind;
use poem::http::StatusCode;

#[derive(Clone)]
struct AppConfig {
    db_host: String,
    db_port: String,
    db_name: String,
    db_user: String,
    db_password: String,
}
#[derive(FromRow)]
struct PiApiToken {
    id: i64,
    userid: i64,
    keyval: Vec<u8>,
    secretval: Vec<u8>,
    #[allow(dead_code)]
    permlevel: i32,
    #[allow(dead_code)]
    rate_limited: i32,
}

#[derive(SecurityScheme)]
#[oai(ty = "api_key", key_name = "Authorization", key_in = "header", checker = "auth_checker")]
struct MySecurityScheme1(ApiKey);

#[derive(SecurityScheme)]
enum MySecurityScheme {
    MySecurityScheme1(MySecurityScheme1),
}

static GLOBAL_MAP: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    Mutex::new(m)
});

struct Api;
#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn hello(
        &self,
        state: Data<&AppConfig>,
        _auth: MySecurityScheme,
    ) -> PlainText<String>
    {
        match db_connect(state.clone()).await {
            Ok(_) => {
                println!("Connected to database");
            }
            Err(err) => {
                eprintln!("Error connecting to database: {}", err);
            }
        }
        PlainText(format!("hello, world!"))
    }

    #[oai(path = "/goodbye", method = "get")]
    async fn goodbye(
        &self,
        state: Data<&AppConfig>,
        _auth: MySecurityScheme,
        name: Query<Option<String>>,
    ) -> PlainText<String>
    {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {

    //Get what version we are
    let version = env!("CARGO_PKG_VERSION");
    println!("Version: {}", version);
    println!("--------------------");

    // Load the .env file
    let mut app_config = AppConfig {
        db_host: "".to_string(),
        db_port: "".to_string(),
        db_name: "".to_string(),
        db_user: "".to_string(),
        db_password: "".to_string(),
    };
    load_app_config(&mut app_config);

    //Load the guid lookup table
    if refresh_kv_apikeys(app_config.clone()).await.is_err() {
        eprintln!("Could not load the guid list from file.");
        std::process::exit(1);
    }

    let api_service =
        OpenApiService::new(
            Api,
            "Hello World",
            "1.0",
        ).server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    let app = Route::new()
        .nest("/api/2.0", api_service)
        .data(app_config);


    poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}

fn load_app_config(piapi_config: &mut AppConfig) {
    dotenv().ok();
    if let Ok(envalue) = env::var("PIAPI_DB_HOST") {
        println!("PIAPI_DB_HOST: {}", envalue);
        piapi_config.db_host = envalue;
    } else {
        eprintln!("PIAPI_DB_HOST is not set. Check your .env file!");
        std::process::exit(1);
    }
    if let Ok(envalue) = env::var("PIAPI_DB_PORT") {
        println!("PIAPI_DB_PORT: {}", envalue);
        piapi_config.db_port = envalue;
    } else {
        eprintln!("PIAPI_DB_PORT is not set. Check your .env file!");
        std::process::exit(2);
    }
    if let Ok(envalue) = env::var("PIAPI_DB_NAME") {
        println!("PIAPI_DB_NAME: {}", envalue);
        piapi_config.db_name = envalue;
    } else {
        eprintln!("PIAPI_DB_NAME is not set. Check your .env file!");
        std::process::exit(3);
    }
    if let Ok(envalue) = env::var("PIAPI_DB_USER") {
        println!("PIAPI_DB_USER: {}", envalue);
        piapi_config.db_user = envalue;
    } else {
        eprintln!("PIAPI_DB_USER is not set. Check your .env file!");
        std::process::exit(4);
    }
    if let Ok(envalue) = env::var("PIAPI_DB_PASSWORD") {
        println!("PIAPI_DB_PASSWORD: {}", envalue);
        piapi_config.db_password = envalue;
    } else {
        eprintln!("PIAPI_DB_PASSWORD is not set. Check your .env file!");
        std::process::exit(5);
    }
}

async fn db_connect(app_config: AppConfig) -> Result<Pool<MySql>, Error> {
    return MySqlPool::connect(
        format!(
            "mysql://{}:{}@{}:{}/{}",
            app_config.db_user,
            app_config.db_password,
            app_config.db_host,
            app_config.db_port,
            app_config.db_name
        ).as_str()
    ).await;
}

async fn refresh_kv_apikeys(
    app_config: AppConfig
)
    -> Result<bool, std::io::Error>
{
    match db_connect(app_config).await {
        Ok(pool) => {
            let query_result = sqlx::query_as::<_, PiApiToken>(
                "SELECT id, userid, keyval, secretval, permlevel, rate_limited FROM api_tokens"
            ).fetch_all(&pool).await.unwrap();

            println!("Number of API tokens loaded: {}", query_result.len());

            for (rindex, api_token) in query_result.iter().enumerate() {
                println!("{}. No.: {}, UserID: {}, KeyVal: {}",
                         rindex + 1,
                         &api_token.id,
                         &api_token.userid,
                         String::from_utf8(api_token.keyval.clone()).unwrap(),
                );

                let keyval = String::from_utf8(api_token.keyval.clone()).unwrap();
                let secretval = String::from_utf8(api_token.secretval.clone()).unwrap();

                let global_map = GLOBAL_MAP.lock();
                match global_map {
                    Ok(mut map) => {
                        map.insert(
                            keyval,
                            secretval,
                        );
                    }
                    Err(_) => {
                        return Err(std::io::Error::new(ErrorKind::Other, "oh no!"));
                    }
                }
            }

            Ok(true)
        }
        Err(err) => {
            eprintln!("Error connecting to database: {}", err);
            std::process::exit(1);
        }
    }
}

async fn auth_checker(request: &&Request, api_key: ApiKey) -> poem::Result<ApiKey> {
    println!("Checking auth: [{:#?}].", api_key);

    let global_map = GLOBAL_MAP.lock();
    match global_map {
        Ok(mut map) => {
            if map.get(&api_key.key).is_some() {
                Ok(ApiKey::from(api_key))
            } else {
                Err(
                    poem::error::Error::from_string(
                        "Api token not found.",
                        StatusCode::UNAUTHORIZED,
                    )
                )
            }
        }
        Err(_) => {
            Err(
                poem::error::Error::from_string(
                    "Api token not found.",
                    StatusCode::UNAUTHORIZED,
                )
            )
        }
    }
}

#[allow(dead_code)]
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}