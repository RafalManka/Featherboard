#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Idea {
    pub id: i64,
    pub org_id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
}

impl Idea {
    pub fn status_label(&self) -> &'static str {
        IdeaStatus::parse(&self.status)
            .map(IdeaStatus::label)
            .unwrap_or("Unknown")
    }

    pub fn status_class(&self) -> &'static str {
        IdeaStatus::parse(&self.status)
            .map(IdeaStatus::css_class)
            .unwrap_or("status-unknown")
    }

    pub fn created_date(&self) -> &str {
        self.created_at.get(..10).unwrap_or(&self.created_at)
    }

    pub fn excerpt(&self) -> String {
        const MAX_CHARS: usize = 140;
        if self.description.chars().count() <= MAX_CHARS {
            return self.description.clone();
        }
        let truncated: String = self.description.chars().take(MAX_CHARS).collect();
        format!("{}…", truncated.trim_end())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IdeaStatus {
    Open,
    Planned,
    InProgress,
    Done,
    Declined,
}

impl IdeaStatus {
    pub const ALL: [IdeaStatus; 5] = [
        Self::Open,
        Self::Planned,
        Self::InProgress,
        Self::Done,
        Self::Declined,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "planned" => Some(Self::Planned),
            "in_progress" => Some(Self::InProgress),
            "done" => Some(Self::Done),
            "declined" => Some(Self::Declined),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Declined => "declined",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Planned => "Planned",
            Self::InProgress => "In Progress",
            Self::Done => "Done",
            Self::Declined => "Declined",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Open => "status-open",
            Self::Planned => "status-planned",
            Self::InProgress => "status-in-progress",
            Self::Done => "status-done",
            Self::Declined => "status-declined",
        }
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Comment {
    pub id: i64,
    pub idea_id: i64,
    pub parent_comment_id: Option<i64>,
    pub author_name: String,
    pub body: String,
    pub created_at: String,
}
