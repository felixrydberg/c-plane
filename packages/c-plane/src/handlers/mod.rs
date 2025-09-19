use actix_web::web;

mod organisations;
mod health;
mod hooks;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .configure(organisations::config)
        .configure(health::config)
        .configure(hooks::config);
}
