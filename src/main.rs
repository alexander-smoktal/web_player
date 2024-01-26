use std::{collections::HashMap, path::PathBuf};

use actix_files::Files;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{self, Key},
    error,
    http::StatusCode,
    middleware,
    web::{self, Redirect},
    App, Error, HttpMessage, HttpRequest, HttpResponse, HttpServer,
};
use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use serde::Deserialize;
use serde_json::json;
use tinytemplate::TinyTemplate;

use crate::file_browser::VIDEO_URL_PREFIX;

mod file_browser;
mod utils;

/// Simple web video player
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short = 'v', long)]
    path_to_videos: PathBuf,

    /// Number of times to greet
    #[arg(short, long, default_value_t = LevelFilter::Debug)]
    log_level: LevelFilter,

    /// HTTP auth login
    #[arg(short, long)]
    user: String,

    /// HTTP auth password
    #[arg(short, long)]
    password: String,
}

struct AppData<'a> {
    templates: TinyTemplate<'a>,
    videos_path: PathBuf,
    user: String,
    password: String,
}

#[derive(Deserialize, Debug)]
struct LoginData {
    user: String,
    password: String,
}

static FILE_TREE: &str = include_str!("../templates/file_tree.html");
static FOOTER: &str = include_str!("../templates/footer.html");
static HEADER: &str = include_str!("../templates/header.html");
static INDEX: &str = include_str!("../templates/index.html");
static LOGIN: &str = include_str!("../templates/login.html");

async fn index(
    data: web::Data<AppData<'_>>,
    query: web::Query<HashMap<String, String>>,
    identity: Option<Identity>,
) -> Result<HttpResponse, Error> {
    // Check if user is logged in
    if identity.is_none() {
        return login_form(&data);
    }

    let file_list = file_browser::browse_dir(
        &data.videos_path,
        &data.videos_path,
        query.get("video_path"),
    )
    .map_err(|_| error::ErrorInternalServerError("File list error"))?;

    let context = if let Some(video_path) = query.get("video_path") {
        json!({
            "files": file_list,
            "video_path" : video_path,
            "no_value": null
        })
    } else {
        json!({
            "files": file_list,
            "video_path" : false,
            "no_value": null
        })
    };

    let html = data.templates.render("index.html", &context).map_err(|e| {
        error::ErrorInternalServerError(format!("Template error {}", e.to_string()))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

fn login_form(data: &web::Data<AppData<'_>>) -> Result<HttpResponse, Error> {
    let html = data
        .templates
        .render(
            "login.html",
            &json!({
                "no_value": null
            }),
        )
        .map_err(|e| {
            error::ErrorInternalServerError(format!("Template error {}", e.to_string()))
        })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn login(
    req: HttpRequest,
    data: web::Data<AppData<'_>>,
    params: web::Form<LoginData>,
) -> Redirect {
    if params.user == data.user && params.password == data.password {
        Identity::login(&req.extensions(), data.user.clone()).unwrap();
    }

    web::Redirect::to("/").using_status_code(StatusCode::FOUND)
}

async fn logout(id: Option<Identity>) -> Redirect {
    if let Some(id) = id {
        id.logout();
    }

    web::Redirect::to("/").using_status_code(StatusCode::FOUND)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(args.log_level)
        .init();

    log::info!("starting HTTP server at http://localhost:8080");
    log::debug!("Current dir: {:?}", std::env::current_dir().unwrap());

    HttpServer::new(move || {
        let files_path = args.path_to_videos.clone();
        let file_path_str = files_path.to_str().unwrap().to_owned();

        let mut templates = TinyTemplate::new();
        templates.add_template("header.html", HEADER).unwrap();
        templates.add_template("footer.html", FOOTER).unwrap();
        templates.add_template("index.html", INDEX).unwrap();
        templates.add_template("file_tree.html", FILE_TREE).unwrap();
        templates.add_template("login.html", LOGIN).unwrap();

        App::new()
            .app_data(web::Data::new(AppData {
                videos_path: files_path,
                templates,
                user: args.user.clone(),
                password: args.password.clone(),
            }))
            .wrap(middleware::Logger::default())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    .cookie_secure(false)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(cookie::time::Duration::days(7)),
                    )
                    .build(),
            )
            .service(web::resource("/").route(web::get().to(index)))
            .service(Files::new("/static", "static/"))
            .service(web::redirect("/favicon.ico", "/static/favicon.ico"))
            .service(Files::new(VIDEO_URL_PREFIX, &file_path_str))
            .service(web::resource("/login").route(web::post().to(login)))
            .service(web::resource("/logout").route(web::get().to(logout)))
            .service(web::scope("").wrap(utils::error_handlers()))
    })
    .bind(("0.0.0.0", 8080))?
    .workers(1)
    .run()
    .await
}
