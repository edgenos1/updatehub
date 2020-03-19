// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use crate::states::actor;
use actix_web::{http::StatusCode, web, HttpRequest, HttpResponse, Responder};
use sdk::api;
use slog_scope::debug;
use thiserror::Error;

pub(crate) struct API(actix::Addr<actor::Machine>);

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Mailbox error: {0}")]
    ActixMailbox(#[from] actix::MailboxError),
}

impl API {
    pub(crate) fn configure(cfg: &mut web::ServiceConfig, addr: actix::Addr<actor::Machine>) {
        cfg.data(Self(addr))
            .route("/info", web::get().to(API::info))
            .route("/log", web::get().to(API::log))
            .route("/probe", web::post().to(API::probe))
            .route("/local_install", web::post().to(API::local_install))
            .route("/remote_install", web::post().to(API::remote_install))
            .route("/update/download/abort", web::post().to(API::download_abort));
    }

    async fn info(agent: web::Data<API>) -> Result<HttpResponse> {
        debug!("Receiving info request");
        Ok(HttpResponse::Ok().json(agent.0.send(actor::info::Request).await?))
    }

    async fn probe(
        agent: web::Data<API>,
        server_address: Option<web::Json<api::probe::Request>>,
    ) -> Result<actor::probe::Response> {
        let server_address = server_address.map(|r| r.into_inner().custom_server);
        debug!("Receiving probe request with {:?}", server_address);
        Ok(agent.0.send(actor::probe::Request(server_address)).await?)
    }

    async fn local_install(
        agent: web::Data<API>,
        file_path: String,
    ) -> Result<actor::local_install::Response> {
        debug!("Receiving local_install request with {:?}", file_path);
        Ok(agent.0.send(actor::local_install::Request(std::path::PathBuf::from(file_path))).await?)
    }

    async fn remote_install(
        agent: web::Data<API>,
        url: String,
    ) -> Result<actor::remote_install::Response> {
        debug!("Receiving remote_install request with {:?}", url);
        Ok(agent.0.send(actor::remote_install::Request(url)).await?)
    }

    async fn log() -> HttpResponse {
        debug!("Receiving log request");
        HttpResponse::Ok().json(crate::logger::buffer())
    }

    async fn download_abort(agent: web::Data<API>) -> Result<actor::download_abort::Response> {
        debug!("Receiving abort download request");
        Ok(agent.0.send(actor::download_abort::Request).await?)
    }
}

impl Responder for actor::download_abort::Response {
    type Error = actix_web::Error;
    type Future = HttpResponse;

    fn respond_to(self, _: &HttpRequest) -> Self::Future {
        match self {
            actor::download_abort::Response::RequestAccepted => {
                HttpResponse::Ok().json(api::abort_download::Response {
                    message: "request accepted, download aborted".to_owned(),
                })
            }
            actor::download_abort::Response::InvalidState => {
                HttpResponse::BadRequest().json(api::abort_download::Refused {
                    error: "there is no download to be aborted".to_owned(),
                })
            }
        }
    }
}

impl Responder for actor::probe::Response {
    type Error = actix_web::Error;
    type Future = HttpResponse;

    fn respond_to(self, _: &HttpRequest) -> Self::Future {
        match self {
            actor::probe::Response::RequestAccepted(current_state) => {
                HttpResponse::Ok().json(api::state::Response { busy: false, current_state })
            }
            actor::probe::Response::InvalidState(current_state) => {
                HttpResponse::Ok().json(api::state::Response { busy: true, current_state })
            }
        }
    }
}

impl Responder for actor::local_install::Response {
    type Error = actix_web::Error;
    type Future = HttpResponse;

    fn respond_to(self, _: &HttpRequest) -> Self::Future {
        match self {
            actor::local_install::Response::RequestAccepted(current_state) => {
                HttpResponse::Ok().json(api::state::Response { busy: false, current_state })
            }
            actor::local_install::Response::InvalidState(current_state) => {
                HttpResponse::UnprocessableEntity()
                    .json(api::state::Response { busy: true, current_state })
            }
        }
    }
}

impl Responder for actor::remote_install::Response {
    type Error = actix_web::Error;
    type Future = HttpResponse;

    fn respond_to(self, _: &HttpRequest) -> Self::Future {
        match self {
            actor::remote_install::Response::RequestAccepted(current_state) => {
                HttpResponse::Ok().json(api::state::Response { busy: false, current_state })
            }
            actor::remote_install::Response::InvalidState(current_state) => {
                HttpResponse::UnprocessableEntity()
                    .json(api::state::Response { busy: true, current_state })
            }
        }
    }
}

impl actix_web::ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError().finish()
    }
}
