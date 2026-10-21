use super::Ctx;
use crate::{
    models::{self, Errors, application, request},
    routes::paths,
};
use askama::Template;

#[derive(Debug, Clone)]
pub struct TaskView {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub agent: String,
    pub category: String,
    pub location: String,
    pub budget: String,
    pub status: String,
    pub open: bool,
    pub date: String,
}
impl TaskView {
    pub fn new(task: &request::Model, agent: Option<&models::Agent>) -> Self {
        Self {
            id: task.id,
            title: task.title.clone(),
            description: task.description.clone(),
            agent: agent.map_or_else(|| format!("Agent #{}", task.agent_id), |a| a.name.clone()),
            category: request::CATEGORIES
                .iter()
                .find(|(k, _)| *k == task.category)
                .map_or("Other help", |(_, label)| *label)
                .into(),
            location: task.location.clone(),
            budget: if task.budget_cents == 0 {
                "Volunteer".into()
            } else {
                format!("${}", request::money_to_string(task.budget_cents))
            },
            status: task.status.label().into(),
            open: task.status == request::RequestStatus::Open,
            date: task.created_at.format("%b %d").to_string(),
        }
    }
    pub fn href(&self) -> String {
        paths::task(self.id)
    }
}

#[derive(Template)]
#[template(path = "board/index.html")]
pub struct BoardIndex {
    pub ctx: Ctx,
    pub tasks: Vec<TaskView>,
    pub filter: request::BoardFilter,
    pub page: u64,
    pub more: bool,
}
impl BoardIndex {
    pub fn categories(&self) -> &'static [(&'static str, &'static str)] {
        request::CATEGORIES
    }
    pub fn page_url(&self, page: u64) -> String {
        let query = serde_html_form::to_string([
            ("q", self.filter.q.as_str()),
            ("category", self.filter.category.as_str()),
            ("location", self.filter.location.as_str()),
        ])
        .unwrap_or_default();
        format!("/?{query}&page={page}")
    }
}

#[derive(Template)]
#[template(path = "board/show.html")]
pub struct TaskDetail {
    pub ctx: Ctx,
    pub task: TaskView,
    pub owner: bool,
    pub applied: bool,
    pub params: application::ApplicationParams,
    pub errors: Errors,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ApplicationView {
    pub id: i64,
    pub request_id: i64,
    pub pitch: String,
    pub contact: String,
    pub selected: bool,
    pub created_at: String,
}
impl From<&application::Model> for ApplicationView {
    fn from(a: &application::Model) -> Self {
        Self {
            id: a.id,
            request_id: a.request_id,
            pitch: a.pitch.clone(),
            contact: a.contact.clone(),
            selected: a.selected_at.is_some(),
            created_at: a.created_at.format("%b %d, %Y").to_string(),
        }
    }
}

#[derive(Template)]
#[template(path = "board/applications.html")]
pub struct ApplicationsIndex {
    pub ctx: Ctx,
    pub task: TaskView,
    pub applications: Vec<ApplicationView>,
    pub page: u64,
    pub more: bool,
}
impl ApplicationsIndex {
    pub fn page_url(&self, page: u64) -> String {
        format!("{}?page={page}", paths::task_applications(self.task.id))
    }
}

pub struct SubmittedView {
    pub application: ApplicationView,
    pub task: TaskView,
}
#[derive(Template)]
#[template(path = "board/dashboard.html")]
pub struct Dashboard {
    pub ctx: Ctx,
    pub tasks: Vec<TaskView>,
    pub applications: Vec<SubmittedView>,
    pub page: u64,
    pub more: bool,
}

#[derive(Template)]
#[template(path = "board/guide.html")]
pub struct AgentGuide {
    pub ctx: Ctx,
}
