use crate::{
    entry::{
        Entry,
        Metadata,
    },
    store::Store,
    store_csv::CsvStore,
    templating,
};
use chrono::Utc;
use failure::Error;
use http::{
    response::Response,
    StatusCode,
};
use serde::Deserialize;
use tera::Tera;
use tide::{
    error::ResultExt,
    forms::ExtractForms,
    response,
    Context,
    EndpointResult,
};
use uuid::Uuid;

pub(super) struct WebService {
    store: CsvStore,
    templates: Tera,
}

impl WebService {
    pub(super) fn open(store: CsvStore) -> Result<Self, Error> {
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

    pub(super) fn run(self, binding: std::net::SocketAddr) -> Result<(), Error> {
        let mut app = tide::App::with_state(self);

        app.middleware(tide::middleware::RootLogger::new());

        app.at("/").get(handler_index);

        app.at("/project/:project").get(handler_project);
        app.at("/project/add/entry/:project")
            .get(handler_project_add_entry);
        app.at("/entry/:uuid").get(handler_entry);
        app.at("/entry/edit/:uuid").get(handler_entry_edit);
        app.at("/entry/move_project/:uuid").get(handler_entry_move_project);

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

        Ok(app.serve(binding)?)
    }
}

async fn handler_index(context: Context<WebService>) -> EndpointResult {
    let mut projects_count = context
        .state()
        .store
        .get_projects_count()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();

    projects_count.sort();

    let mut template_context = tera::Context::new();
    template_context.insert("projects_count", &projects_count);

    let output = context
        .state()
        .templates
        .render("index.html", &template_context)
        .unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_project(context: Context<WebService>) -> EndpointResult {
    let project: String = context.param("project").client_err()?;

    let show_done = match context.uri().query() {
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

    let entries_active = context.state().store.get_active_entries(&project).unwrap();
    let entries_done = if show_done {
        context.state().store.get_done_entries(&project).unwrap()
    } else {
        crate::entry::Entries::default()
    };

    let mut template_context = tera::Context::new();
    template_context.insert("entries_active", &entries_active.into_inner());
    template_context.insert("entries_done", &entries_done.into_inner());
    template_context.insert("project", &project);
    template_context.insert("show_done", &show_done);

    let output = context
        .state()
        .templates
        .render("project.html", &template_context)
        .unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_project_add_entry(context: Context<WebService>) -> EndpointResult {
    let project: String = context
        .param("project")
        .client_err()
        .unwrap_or_else(|_| "work".to_string());

    let mut template_context = tera::Context::new();
    template_context.insert("project", &project);

    let output = context
        .state()
        .templates
        .render("project_add_entry.html", &template_context)
        .unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_entry(context: Context<WebService>) -> EndpointResult {
    let uuid: uuid::Uuid = match context.param("uuid").client_err() {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body("500 - no uuid found".into())
                .unwrap())
        }
    };

    let entry = context.state().store.get_entry_by_uuid(&uuid).unwrap();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);

    let output = context
        .state()
        .templates
        .render("entry.html", &template_context)
        .unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_entry_edit(context: Context<WebService>) -> EndpointResult {
    let uuid: uuid::Uuid = match context.param("uuid").client_err() {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body("500 - no uuid found".into())
                .unwrap())
        }
    };

    let entry = context.state().store.get_entry_by_uuid(&uuid).unwrap();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);

    let output = context
        .state()
        .templates
        .render("entry_edit.html", &template_context)
        .unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_entry_move_project(context: Context<WebService>) -> EndpointResult {
    let uuid: uuid::Uuid = match context.param("uuid").client_err() {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body("500 - no uuid found".into())
                .unwrap())
        }
    };

    let entry = context.state().store.get_entry_by_uuid(&uuid).unwrap();
    let mut projects = context.state().store.get_projects().unwrap();
    projects.sort();
    projects.dedup();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);
    template_context.insert("projects", &projects);

    let output = context
        .state()
        .templates
        .render("entry_move_project.html", &template_context)
        .unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_api_v1_project_entries(context: Context<WebService>) -> EndpointResult {
    let project: String = context.param("project").client_err()?;

    let entries = context.state().store.get_active_entries(&project).unwrap();

    Ok(response::json(entries))
}

async fn handler_api_v1_mark_entry_done(context: Context<WebService>) -> EndpointResult {
    let uuid: Uuid = context.param("uuid").client_err()?;

    context.state().store.entry_done_by_uuid(uuid).unwrap();

    let location = format!("/entry/{}", uuid);

    Ok(Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Content-Type", "text/plain")
        .header("Location", location)
        .body("entry updated to be done".into())
        .unwrap())
}

async fn handler_api_v1_mark_entry_active(context: Context<WebService>) -> EndpointResult {
    let uuid: Uuid = context.param("uuid").client_err()?;

    context.state().store.entry_active_by_uuid(uuid).unwrap();

    let location = format!("/entry/{}", uuid);

    Ok(Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Content-Type", "text/plain")
        .header("Location", location)
        .body("entry updated to be active".into())
        .unwrap())
}

async fn handler_api_v1_project_add_entry(mut context: Context<WebService>) -> EndpointResult {
    #[derive(Deserialize, Debug)]
    struct Message {
        text: String,
    }

    let project: String = context.param("project").client_err()?;
    let message: Message = context.body_form().await?;

    let entry = Entry {
        text: message.text.replace("\r", ""),
        metadata: Metadata {
            project,
            ..Metadata::default()
        },
    };

    let uuid = entry.metadata.uuid;

    context.state().store.add_entry(entry).unwrap();

    Ok(Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Content-Type", "text/plain")
        .header("Location", format!("/entry/{}", uuid))
        .body("entry updated to be done".into())
        .unwrap())
}

async fn handler_api_v1_entry_edit(mut context: Context<WebService>) -> EndpointResult {
    #[derive(Deserialize, Debug)]
    struct Message {
        text: String,
        update_time: Option<String>,
    }

    let uuid: uuid::Uuid = match context.param("uuid").client_err() {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body("500 - no uuid found".into())
                .unwrap())
        }
    };

    let message: Message = context.body_form().await?;

    let old_entry = context.state().store.get_entry_by_uuid(&uuid).unwrap();

    let new_entry = if message.update_time.is_some() {
        Entry {
            text: message.text,
            metadata: Metadata {
                started: Utc::now(),
                last_change: Utc::now(),
                ..old_entry.metadata
            },
        }
    } else {
        Entry {
            text: message.text,
            ..old_entry
        }
    };

    context.state().store.update_entry(new_entry).unwrap();

    Ok(Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Content-Type", "text/plain")
        .header("Location", format!("/entry/{}", uuid))
        .body("entry text updated".into())
        .unwrap())
}

async fn handler_api_v1_entry_move_project(mut context: Context<WebService>) -> EndpointResult {
    #[derive(Deserialize, Debug)]
    struct Message {
        new_project: String,
    }

    let message: Message = context.body_form().await?;

    let uuid: uuid::Uuid = match context.param("uuid").client_err() {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body("500 - no uuid found".into())
                .unwrap())
        }
    };

    let old_entry = context.state().store.get_entry_by_uuid(&uuid).unwrap();

    let new_entry = Entry {
            metadata: Metadata {
                project: message.new_project,
                last_change: Utc::now(),
                ..old_entry.metadata
            },
            ..old_entry
        };

    context.state().store.update_entry(new_entry).unwrap();

    Ok(Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Content-Type", "text/plain")
        .header("Location", format!("/entry/{}", uuid))
        .body("entry text updated".into())
        .unwrap())
}

async fn handler_static_css_main(_context: Context<WebService>) -> EndpointResult {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_bytes!("resources/css/main.css").to_vec().into())
        .unwrap())
}

async fn handler_static_css_font_awesome(_context: Context<WebService>) -> EndpointResult {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(
            include_bytes!("resources/css/font-awesome.min.css")
                .to_vec()
                .into(),
        )
        .unwrap())
}

async fn handler_static_fonts_fontawesome_webfont_woff2(
    _context: Context<WebService>,
) -> EndpointResult {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "font/woff2")
        .body(
            include_bytes!("resources/fonts/fontawesome-webfont.woff2")
                .to_vec()
                .into(),
        )
        .unwrap())
}

async fn handler_favicon_ico(_context: Context<WebService>) -> EndpointResult {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/x-icon")
        .body(include_bytes!("resources/img/favicon.ico").to_vec().into())
        .unwrap())
}
