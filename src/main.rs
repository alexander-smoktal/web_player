use std::{collections::HashMap, path::PathBuf};

use actix_files::Files;
use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    error,
    http::{header::ContentType, StatusCode},
    middleware::{self, ErrorHandlerResponse, ErrorHandlers},
    web, App, Error, HttpResponse, HttpServer,
};
use actix_web_httpauth::{extractors::basic::BasicAuth, middleware::HttpAuthentication};
use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use serde_json::json;
use tinytemplate::TinyTemplate;

mod file_browser;

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
    files_path: PathBuf,
}

static INDEX: &str = include_str!("../templates/index.html");
static FILE_TREE: &str = include_str!("../templates/file_tree.html");

async fn index(
    data: web::Data<AppData<'_>>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let file_list = file_browser::browse_dir(&data.files_path, query.get("video_path"))
        .map_err(|_| error::ErrorInternalServerError("File list error"))?;

    let s = if let Some(video_path) = query.get("video_path") {
        // submitted form
        let ctx = json!({
            "files": file_list,
            "video_path" : video_path,
        });
        data.templates.render("index.html", &ctx).map_err(|e| {
            error::ErrorInternalServerError(format!("Template error {}", e.to_string()))
        })?
    } else {
        data.templates
            .render(
                "index.html",
                &json!({
                    "files": file_list,
                    "video_path" : false,
                }),
            )
            .map_err(|e| {
                error::ErrorInternalServerError(format!("Template error {}", e.to_string()))
            })?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

async fn auth_validator(
    req: ServiceRequest,
    credentials: BasicAuth,
    login: String,
    password: String,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    if credentials.user_id() == login
        && credentials.password().is_some()
        && credentials.password().unwrap() == password
    {
        Ok(req)
    } else {
        Err((error::ErrorForbidden("Unautorized"), req))
    }
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
        let login_c = args.user.clone();
        let password_c = args.password.clone();

        let mut templates = TinyTemplate::new();
        templates.add_template("file_tree.html", FILE_TREE).unwrap();
        templates.add_template("index.html", INDEX).unwrap();

        App::new()
            .app_data(web::Data::new(AppData {
                files_path,
                templates,
            }))
            .wrap(middleware::Logger::default())
            .wrap(HttpAuthentication::basic(move |req, creds| {
                let login = login_c.clone();
                let password = password_c.clone();
                async { auth_validator(req, creds, login, password).await }
            }))
            .service(web::resource("/").route(web::get().to(index)))
            .service(Files::new("/static", "static/"))
            .service(web::redirect("/favicon.ico", "/static/favicon.ico"))
            .service(Files::new(&file_path_str, &file_path_str))
            .service(web::scope("").wrap(error_handlers()))
    })
    .bind(("0.0.0.0", 8080))?
    .workers(1)
    .run()
    .await
}

// Custom error handlers, to return HTML responses when an error occurs.
fn error_handlers() -> ErrorHandlers<BoxBody> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

// Error handler for a 404 Page not found error.
fn not_found<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<BoxBody>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        res.into_parts().0,
        response.map_into_left_body(),
    )))
}

// Generic error handler.
fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> HttpResponse {
    let request = res.request();

    // Provide a fallback to a simple plain text response in case an error occurs during the
    // rendering of the error page.
    let fallback = |err: &str| {
        HttpResponse::build(res.status())
            .content_type(ContentType::plaintext())
            .body(err.to_string())
    };

    let tt = request
        .app_data::<web::Data<TinyTemplate<'_>>>()
        .map(|t| t.get_ref());
    match tt {
        Some(tt) => {
            let mut context = std::collections::HashMap::new();
            context.insert("error", error.to_owned());
            context.insert("status_code", res.status().as_str().to_owned());
            let body = tt.render("error.html", &context);

            match body {
                Ok(body) => HttpResponse::build(res.status())
                    .content_type(ContentType::html())
                    .body(body),
                Err(_) => fallback(error),
            }
        }
        None => fallback(error),
    }
}
