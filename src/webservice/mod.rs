use crate::{
    entry::{
        Entry,
        Metadata,
    },
    store::Store,
    templating,
};
use chrono::Utc;
use failure::Error;
use http_types::mime;
use serde::Deserialize;
use tera::Tera;
use tide::{
    Body,
    Request,
    Response,
    StatusCode,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub(super) struct WebService {
    store: Store,
    templates: Tera,
}

impl WebService {
    pub(super) fn open(store: Store) -> Result<Self, Error> {
        let templates = WebService::open_templates()?;

        Ok(Self { store, templates })
    }

    fn open_templates() -> Result<Tera, Error> {
        let mut templates = tera::Tera::default();

        let index_raw = include_str!("resources/html/index.html.tera");
        templates.add_raw_template("index.html", index_raw).unwrap();

        let project_raw = include_str!("resources/html/project.html.tera");
        templates
            .add_raw_template("project.html", project_raw)
            .unwrap();

        let entry_raw = include_str!("resources/html/entry.html.tera");
        templates.add_raw_template("entry.html", entry_raw).unwrap();

        let entry_edit_raw = include_str!("resources/html/entry_edit.html.tera");
        templates
            .add_raw_template("entry_edit.html", entry_edit_raw)
            .unwrap();

        let entry_move_project_raw = include_str!("resources/html/entry_move_project.html.tera");
        templates
            .add_raw_template("entry_move_project.html", entry_move_project_raw)
            .unwrap();

        let project_add_entry_raw = include_str!("resources/html/project_add_entry.html.tera");
        templates
            .add_raw_template("project_add_entry.html", project_add_entry_raw)
            .unwrap();

        templates.register_filter("asciidoc_header", templating::asciidoc_header);
        templates.register_filter("asciidoc_to_html", templating::asciidoc_to_html);
        templates.register_filter("format_duration_since", templating::format_duration_since);
        templates.register_filter("lines", templating::lines);
        templates.register_filter("single_line", templating::single_line);
        templates.register_filter("some_or_dash", templating::some_or_dash);

        templates.register_tester("some", templating::some);

        Ok(templates)
    }

    pub(super) async fn run(self, binding: std::net::SocketAddr) -> Result<(), Error> {
        let mut app = tide::with_state(self);

        app.at("/").get(handler_index);
        app.at("/_/health").get(handler_health);
        app.at("/_/health").options(handler_health);

        app.at("/project/:project").get(handler_project);
        app.at("/project/add/entry/:project")
            .get(handler_project_add_entry);
        app.at("/entry/:uuid").get(handler_entry);
        app.at("/entry/edit/:uuid").get(handler_entry_edit);
        app.at("/entry/move_project/:uuid")
            .get(handler_entry_move_project);

        app.at("/api/v1/project/entries/:project")
            .get(handler_api_v1_project_entries);
        app.at("/api/v1/entry/mark/done/:uuid")
            .get(handler_api_v1_mark_entry_done);
        app.at("/api/v1/entry/mark/active/:uuid")
            .get(handler_api_v1_mark_entry_active);
        app.at("/api/v1/project/add/entry/:project")
            .post(handler_api_v1_project_add_entry);
        app.at("/api/v1/entry/edit/:uuid")
            .post(handler_api_v1_entry_edit);
        app.at("/api/v1/entry/move_project/:uuid")
            .post(handler_api_v1_entry_move_project);

        app.at("/static/css/main.css").get(handler_static_css_main);
        app.at("/static/css/font-awesome.min.css")
            .get(handler_static_css_font_awesome);
        app.at("/static/fonts/fontawesome-webfont.woff2")
            .get(handler_static_fonts_fontawesome_webfont_woff2);

        app.at("/favicon.ico").get(handler_favicon_ico);

        app.listen(binding).await?;

        Ok(())
    }
}

async fn handler_index(request: Request<WebService>) -> Result<Response, tide::Error> {
    let mut projects_count = request
        .state()
        .store
        .get_projects_count()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();

    projects_count.sort();

    let mut template_context = tera::Context::new();
    template_context.insert("projects_count", &projects_count);

    let output = request
        .state()
        .templates
        .render("index.html", &template_context)
        .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/html")
        .body(Body::from(output))
        .build())
}

async fn handler_health(_request: Request<WebService>) -> Result<Response, tide::Error> {
    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/plain")
        .body(Body::from(""))
        .build())
}

async fn handler_project(request: Request<WebService>) -> Result<Response, tide::Error> {
    let project = request.param("project")?;

    // TODO: use request.query() instead
    let show_done = match request.url().query() {
        Some(parameters) => parameters
            .split('&')
            .map(|key_values| {
                let mut split = key_values.split('=');
                (split.next().unwrap_or(""), split.next().unwrap_or(""))
            })
            .find(|(key, _)| key == &"show_done")
            .map(|(_, value)| value.parse().unwrap_or(false))
            .unwrap_or(false),
        None => false,
    };

    let entries_active = request.state().store.get_active_entries(&project).unwrap();
    let entries_done = if show_done {
        request.state().store.get_done_entries(&project).unwrap()
    } else {
        crate::entry::Entries::default()
    };

    let mut template_context = tera::Context::new();
    template_context.insert("entries_active", &entries_active.into_inner());
    template_context.insert("entries_done", &entries_done.into_inner());
    template_context.insert("project", &project);
    template_context.insert("show_done", &show_done);

    let output = request
        .state()
        .templates
        .render("project.html", &template_context)
        .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/html")
        .body(Body::from(output))
        .build())
}

async fn handler_project_add_entry(request: Request<WebService>) -> Result<Response, tide::Error> {
    let project = request.param("project").unwrap_or("work");

    let mut template_context = tera::Context::new();
    template_context.insert("project", &project);

    let output = request
        .state()
        .templates
        .render("project_add_entry.html", &template_context)
        .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/html")
        .body(Body::from(output.as_bytes()))
        .build())
}

async fn handler_entry(request: Request<WebService>) -> Result<Response, tide::Error> {
    let uuid: uuid::Uuid = match request.param("uuid") {
        Ok(uuid) => uuid.parse()?,
        Err(_) => {
            return Ok(Response::builder(StatusCode::InternalServerError)
                .header("Content-Type", "text/plain")
                .body(Body::from("500 - no uuid found"))
                .build())
        }
    };

    let entry = request.state().store.get_entry_by_uuid(&uuid).unwrap();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);

    let output = request
        .state()
        .templates
        .render("entry.html", &template_context)
        .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/html")
        .body(Body::from(output.as_bytes()))
        .build())
}

async fn handler_entry_edit(request: Request<WebService>) -> Result<Response, tide::Error> {
    let uuid: uuid::Uuid = match request.param("uuid") {
        Ok(uuid) => uuid.parse()?,
        Err(_) => {
            return Ok(Response::builder(StatusCode::InternalServerError)
                .header("Content-Type", "text/plain")
                .body(Body::from("500 - no uuid found"))
                .build())
        }
    };

    let entry = request.state().store.get_entry_by_uuid(&uuid).unwrap();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);

    let output = request
        .state()
        .templates
        .render("entry_edit.html", &template_context)
        .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/html")
        .body(Body::from(output.as_bytes()))
        .build())
}

async fn handler_entry_move_project(request: Request<WebService>) -> Result<Response, tide::Error> {
    let uuid: uuid::Uuid = match request.param("uuid") {
        Ok(uuid) => uuid.parse()?,
        Err(_) => {
            return Ok(Response::builder(StatusCode::InternalServerError)
                .header("Content-Type", "text/plain")
                .body(Body::from("500 - no uuid found"))
                .build())
        }
    };

    let entry = request.state().store.get_entry_by_uuid(&uuid).unwrap();
    let mut projects = request.state().store.get_projects().unwrap();
    projects.sort();
    projects.dedup();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);
    template_context.insert("projects", &projects);

    let output = request
        .state()
        .templates
        .render("entry_move_project.html", &template_context)
        .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/html")
        .body(Body::from(output.as_bytes()))
        .build())
}

async fn handler_api_v1_project_entries(
    request: Request<WebService>,
) -> Result<Response, tide::Error> {
    let project = request.param("project")?;

    let entries = request.state().store.get_active_entries(&project).unwrap();

    let response = Response::builder(200)
        .body(Body::from_json(&entries)?)
        .content_type(mime::JSON)
        .build();

    Ok(response)
}

async fn handler_api_v1_mark_entry_done(
    request: Request<WebService>,
) -> Result<Response, tide::Error> {
    let uuid: Uuid = request.param("uuid")?.parse()?;

    request.state().store.entry_done_by_uuid(uuid).unwrap();

    let location = format!("/entry/{}", uuid);

    Ok(Response::builder(StatusCode::SeeOther)
        .header("Content-Type", "text/plain")
        .header("Location", location)
        .body(Body::from("entry updated to be done"))
        .build())
}

async fn handler_api_v1_mark_entry_active(
    request: Request<WebService>,
) -> Result<Response, tide::Error> {
    let uuid: Uuid = request.param("uuid")?.parse()?;

    request.state().store.entry_active_by_uuid(uuid).unwrap();

    let location = format!("/entry/{}", uuid);

    Ok(Response::builder(StatusCode::SeeOther)
        .header("Content-Type", "text/plain")
        .header("Location", location)
        .body(Body::from("entry updated to be active"))
        .build())
}

async fn handler_api_v1_project_add_entry(
    mut request: Request<WebService>,
) -> Result<Response, tide::Error> {
    #[derive(Deserialize, Debug)]
    struct Message {
        text: String,
    }

    let project = request.param("project")?.to_owned();
    let message: Message = request.body_form().await?;

    let entry = Entry {
        text: message.text.replace("\r", ""),
        metadata: Metadata {
            project,
            ..Metadata::default()
        },
    };

    let uuid = entry.metadata.uuid;

    request.state().store.add_entry(entry).unwrap();

    Ok(Response::builder(StatusCode::SeeOther)
        .header("Content-Type", "text/plain")
        .header("Location", format!("/entry/{}", uuid))
        .body(Body::from("entry updated to be done"))
        .build())
}

async fn handler_api_v1_entry_edit(
    mut request: Request<WebService>,
) -> Result<Response, tide::Error> {
    #[derive(Deserialize, Debug)]
    struct Message {
        text: String,
        update_time: Option<String>,
    }

    let uuid: uuid::Uuid = match request.param("uuid") {
        Ok(uuid) => uuid.parse()?,
        Err(_) => {
            return Ok(Response::builder(StatusCode::InternalServerError)
                .header("Content-Type", "text/plain")
                .body(Body::from("500 - no uuid found"))
                .build())
        }
    };

    let message: Message = request.body_form().await?;

    let old_entry = request.state().store.get_entry_by_uuid(&uuid).unwrap();

    let text = message.text.replace("\r", "");

    let new_entry = if message.update_time.is_some() {
        Entry {
            text,
            metadata: Metadata {
                started: Utc::now(),
                last_change: Utc::now(),
                ..old_entry.metadata
            },
        }
    } else {
        Entry { text, ..old_entry }
    };

    request.state().store.update_entry(new_entry).unwrap();

    Ok(Response::builder(StatusCode::SeeOther)
        .header("Content-Type", "text/plain")
        .header("Location", format!("/entry/{}", uuid))
        .body(Body::from("entry text updated"))
        .build())
}

async fn handler_api_v1_entry_move_project(
    mut request: Request<WebService>,
) -> Result<Response, tide::Error> {
    #[derive(Deserialize, Debug)]
    struct Message {
        new_project: String,
    }

    let message: Message = request.body_form().await?;

    let uuid: uuid::Uuid = match request.param("uuid") {
        Ok(uuid) => uuid.parse()?,
        Err(_) => {
            return Ok(Response::builder(StatusCode::InternalServerError)
                .header("Content-Type", "text/plain")
                .body(Body::from("500 - no uuid found"))
                .build())
        }
    };

    let old_entry = request.state().store.get_entry_by_uuid(&uuid).unwrap();

    let new_entry = Entry {
        metadata: Metadata {
            project: message.new_project,
            last_change: Utc::now(),
            ..old_entry.metadata
        },
        ..old_entry
    };

    request.state().store.update_entry(new_entry).unwrap();

    Ok(Response::builder(StatusCode::SeeOther)
        .header("Content-Type", "text/plain")
        .header("Location", format!("/entry/{}", uuid))
        .body(Body::from("entry text updated"))
        .build())
}

async fn handler_static_css_main(_request: Request<WebService>) -> Result<Response, tide::Error> {
    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/css")
        .body(Body::from(
            include_bytes!("resources/css/main.css").to_vec(),
        ))
        .build())
}

async fn handler_static_css_font_awesome(
    _request: Request<WebService>,
) -> Result<Response, tide::Error> {
    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "text/css")
        .body(Body::from(
            include_bytes!("resources/css/font-awesome.min.css").to_vec(),
        ))
        .build())
}

async fn handler_static_fonts_fontawesome_webfont_woff2(
    _request: Request<WebService>,
) -> Result<Response, tide::Error> {
    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "font/woff2")
        .body(Body::from(
            include_bytes!("resources/fonts/fontawesome-webfont.woff2").to_vec(),
        ))
        .build())
}

async fn handler_favicon_ico(_request: Request<WebService>) -> Result<Response, tide::Error> {
    Ok(Response::builder(StatusCode::Ok)
        .header("Content-Type", "image/x-icon")
        .body(Body::from(
            include_bytes!("resources/img/favicon.ico").to_vec(),
        ))
        .build())
}
