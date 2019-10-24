use crate::templating;
use crate::{
    store::Store,
    store_csv::CsvStore,
};
use failure::Error;
use http::{
    response::Response,
    StatusCode,
};
use tide::{
    error::ResultExt,
    response,
    Context,
    EndpointResult,
};
use tera::Tera;

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
        templates.add_raw_template("project.html", project_raw).unwrap();

        let entry_raw = include_str!("resources/html/entry.html.tera");
        templates.add_raw_template("entry.html", entry_raw).unwrap();

        templates.register_filter("single_line", templating::single_line);
        templates.register_filter("lines", templating::lines);
        templates.register_filter("format_duration_since", templating::format_duration_since);

        Ok(templates)
    }

    pub(super) fn run(self, binding: std::net::SocketAddr) -> Result<(), Error> {
        let mut app = tide::App::with_state(self);

        app.at("/").get(handler_index);
        app.at("/project/:project").get(handler_project);
        app.at("/entry/:uuid").get(handler_entry);
        app.at("/api/v1/list/:project").get(handler_api_v1_list);

        Ok(app.run(binding)?)
    }
}

async fn handler_index(context: Context<WebService>) -> EndpointResult {
    let mut projects_count = context.state().store
        .get_projects_count().unwrap()
        .into_iter()
        .collect::<Vec<_>>();

    projects_count.sort();

    let mut template_context = tera::Context::new();
    template_context.insert("projects_count", &projects_count);

    let output = context.state().templates.render("index.html", &template_context).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_project(context: Context<WebService>) -> EndpointResult {
    let project: String = context.param("project").client_err().unwrap_or_else(|_| "work".to_string());
    let entries = context.state().store.get_active_entries(&project).unwrap();

    let mut template_context = tera::Context::new();
    template_context.insert("entries", &entries.into_inner());
    template_context.insert("project", &project);

    let output = context.state().templates.render("project.html", &template_context).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_entry(context: Context<WebService>) -> EndpointResult {
    let uuid: uuid::Uuid = context.param("uuid").client_err().unwrap();
    let entry = context.state().store.get_entry_by_uuid(&uuid).unwrap();

    let mut template_context = tera::Context::new();
    template_context.insert("entry", &entry);

    let output = context.state().templates.render("entry.html", &template_context).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(output.as_bytes().into())
        .unwrap())
}

async fn handler_api_v1_list(context: Context<WebService>) -> EndpointResult {
    let project: String = context.param("project").client_err()?;

    let entries = context.state().store.get_active_entries(&project).unwrap();

    Ok(response::json(entries))
}
