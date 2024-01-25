use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    error,
    http::{header::ContentType, StatusCode},
    middleware::{ErrorHandlerResponse, ErrorHandlers},
    web, Error, HttpResponse,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use tinytemplate::TinyTemplate;

pub async fn auth_validator(
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

// Custom error handlers, to return HTML responses when an error occurs.
pub fn error_handlers() -> ErrorHandlers<BoxBody> {
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
pub fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> HttpResponse {
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
