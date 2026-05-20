use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct AuthStartRequest {
    pub client_info: ClientInfo,
}

#[derive(Serialize, Clone)]
pub struct ClientInfo {
    pub hostname: String,
    pub os: String,
    pub os_version: String,
    pub cli_version: String,
}

#[derive(Deserialize)]
pub struct AuthStartResponse {
    pub device_token: String,
    pub user_code: String,
    pub user_code_display: String,
    pub verify_url: String,
    pub expires_in: u64,
    pub poll_interval: u64,
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PollResponse {
    Pending,
    Denied,
    Expired,
    Approved { access_token: String },
}

#[derive(Deserialize)]
pub struct TasksResponse {
    pub project: ProjectRef,
    pub tasks: Vec<TaskSummary>,
    pub count: u32,
}

#[derive(Deserialize)]
pub struct ProjectRef {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub organization_slug: String,
}

#[derive(Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub code: String,
    pub title: String,
    pub status: String,
    pub energy: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<UserRef>,
    pub primary_document: Option<DocumentRef>,
    pub updated_at: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub parent_code: Option<String>,
}

#[derive(Deserialize)]
pub struct UserRef {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct DocumentRef {
    pub id: String,
    pub code: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct TaskContextResponse {
    pub task: TaskDetail,
    pub documents: TaskDocuments,
}

#[derive(Deserialize)]
pub struct TaskDetail {
    pub id: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub energy: Option<String>,
    pub priority: Option<String>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub assignee: Option<UserRef>,
    pub reporter: Option<UserRef>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub parent_code: Option<String>,
}

#[derive(Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub text: String,
    pub is_done: bool,
}

#[derive(Deserialize)]
pub struct TaskDocuments {
    pub primary: Option<DocumentFull>,
    pub related: Vec<DocumentFull>,
}

#[derive(Deserialize)]
pub struct DocumentFull {
    pub id: String,
    pub code: String,
    #[serde(rename = "type")]
    pub doc_type: String,
    pub title: String,
    pub content_markdown: String,
    pub version: i32,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectListEntry>,
    pub count: u32,
}

#[derive(Deserialize)]
pub struct ProjectListEntry {
    pub organization_slug: String,
    pub organization_name: String,
    pub project_slug: String,
    pub project_name: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct CompleteRequest {
    pub agent_session: Option<AgentSessionDto>,
}

#[derive(Serialize)]
pub struct AgentSessionDto {
    pub agent_code: String,
    pub local_repo: Option<String>,
    pub duration_seconds: u64,
    pub tokens_used: u64,
    pub metadata: serde_json::Value,
}

#[derive(Deserialize)]
pub struct TaskCompleteResponse {
    pub task: TaskCompleted,
    #[serde(default)]
    pub usage_recorded: bool,
}

#[derive(Deserialize)]
pub struct TaskCompleted {
    pub id: String,
    pub code: String,
    pub title: String,
    pub status: String,
    pub completed_at: Option<String>,
    pub updated_at: String,
}
