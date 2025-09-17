use actix_web::web;

mod organisations;
mod health;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(organisations::config)
            .configure(health::config)
    );
}
