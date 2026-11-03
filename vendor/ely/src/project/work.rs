use std::fmt;

use gpui::SharedString;
use jiff::civil::Date;

use crate::{mail::Label, primitives::IconName};

/// Where a task stands in its workflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Backlog,
    Todo,
    InProgress,
    InReview,
    Done,
    Canceled,
}

impl Status {
    pub const ALL: [Status; 6] = [
        Status::Backlog,
        Status::Todo,
        Status::InProgress,
        Status::InReview,
        Status::Done,
        Status::Canceled,
    ];

    pub fn words(self) -> &'static str {
        match self {
            Status::Backlog => "Backlog",
            Status::Todo => "Todo",
            Status::InProgress => "In progress",
            Status::InReview => "In review",
            Status::Done => "Done",
            Status::Canceled => "Canceled",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Status::Backlog => IconName::CircleDashed,
            Status::Todo => IconName::Circle,
            Status::InProgress => IconName::CircleDot,
            Status::InReview => IconName::CircleDotDashed,
            Status::Done => IconName::CircleCheck,
            Status::Canceled => IconName::CircleX,
        }
    }

    /// Done or canceled: nothing more is owed.
    pub fn closed(self) -> bool {
        matches!(self, Status::Done | Status::Canceled)
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Status::Backlog => "backlog",
            Status::Todo => "todo",
            Status::InProgress => "in-progress",
            Status::InReview => "in-review",
            Status::Done => "done",
            Status::Canceled => "canceled",
        }
    }

    pub(crate) fn of_key(key: &str) -> Status {
        Status::ALL
            .into_iter()
            .find(|status| status.key() == key)
            .unwrap_or_else(|| panic!("no status {key}"))
    }
}

/// How urgent a task is, from none to urgent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

impl Priority {
    /// Most urgent first.
    pub const ALL: [Priority; 5] = [
        Priority::Urgent,
        Priority::High,
        Priority::Medium,
        Priority::Low,
        Priority::None,
    ];

    pub fn words(self) -> &'static str {
        match self {
            Priority::None => "No priority",
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
            Priority::Urgent => "Urgent",
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Priority::None => "none",
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
            Priority::Urgent => "urgent",
        }
    }

    pub(crate) fn of_key(key: &str) -> Priority {
        Priority::ALL
            .into_iter()
            .find(|priority| priority.key() == key)
            .unwrap_or_else(|| panic!("no priority {key}"))
    }

    /// Bars lit of three; urgent has a mark of its own.
    pub(crate) fn bars(self) -> usize {
        match self {
            Priority::None => 0,
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High | Priority::Urgent => 3,
        }
    }
}

/// An issue's name within its team: the team's key and a number, as ENG-123.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueId {
    team: SharedString,
    number: u32,
}

impl IssueId {
    /// The key is one to five capital letters, and numbers start at one.
    pub fn new(team: impl Into<SharedString>, number: u32) -> Self {
        let team = team.into();
        let letters = (1..=5).contains(&team.len()) && team.chars().all(|c| c.is_ascii_uppercase());
        assert!(letters, "issue team key {team} is not one to five capitals");
        assert!(number > 0, "issue {team} numbers start at one");
        Self { team, number }
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.team, self.number)
    }
}

/// Someone work can go to.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub key: SharedString,
    pub name: SharedString,
}

impl Person {
    pub fn new(key: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        let key = key.into();
        assert!(!key.is_empty(), "a person needs a key");
        Self {
            key,
            name: name.into(),
        }
    }
}

/// A piece of work: its key, its issue's name if it has one, what it is, where it stands, how urgent, when it starts and is due, who has it, and its labels.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub key: SharedString,
    pub issue: Option<IssueId>,
    pub title: SharedString,
    pub status: Status,
    pub priority: Priority,
    pub start: Option<Date>,
    pub due: Option<Date>,
    pub assignee: Option<Person>,
    pub labels: Vec<Label>,
}

impl Task {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            issue: None,
            title: title.into(),
            status: Status::Todo,
            priority: Priority::None,
            start: None,
            due: None,
            assignee: None,
            labels: Vec::new(),
        }
    }

    pub fn issue(mut self, id: IssueId) -> Self {
        self.issue = Some(id);
        self
    }

    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// The day work begins, for a timeline.
    pub fn starts(mut self, day: Date) -> Self {
        self.start = Some(day);
        self
    }

    pub fn due(mut self, day: Date) -> Self {
        self.due = Some(day);
        self
    }

    pub fn assignee(mut self, person: Person) -> Self {
        self.assignee = Some(person);
        self
    }

    pub fn label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Due before `today` and still open.
    pub fn overdue(&self, today: Date) -> bool {
        !self.status.closed() && self.due.is_some_and(|due| due < today)
    }
}

/// A due day in few words from `today`: Today, Tomorrow, Yesterday, a weekday within the week ahead, else the date, with its year when not this one.
pub(crate) fn due_words(due: Date, today: Date) -> String {
    let days = due.since(today).expect("days between two dates").get_days();
    match days {
        0 => "Today".into(),
        1 => "Tomorrow".into(),
        -1 => "Yesterday".into(),
        2..=6 => due.strftime("%a").to_string(),
        _ if due.year() == today.year() => due.strftime("%b %-d").to_string(),
        _ => due.strftime("%b %-d, %Y").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{IssueId, Priority, Status, Task, due_words};

    #[test]
    fn due_days_read_near_then_far() {
        let today = date(2026, 9, 27);
        let words: Vec<String> = [27, 28, 26, 30]
            .map(|day| due_words(date(2026, 9, day), today))
            .into();
        assert_eq!(words, ["Today", "Tomorrow", "Yesterday", "Wed"]);
        assert_eq!(due_words(date(2026, 10, 4), today), "Oct 4");
        assert_eq!(due_words(date(2026, 9, 20), today), "Sep 20");
        assert_eq!(due_words(date(2027, 1, 8), today), "Jan 8, 2027");
    }

    #[test]
    fn statuses_go_by_their_keys() {
        for status in Status::ALL {
            assert_eq!(Status::of_key(status.key()), status);
        }
        assert!(Status::Canceled.closed() && !Status::InReview.closed());
    }

    #[test]
    fn priorities_rise_and_light_their_bars() {
        assert!(Priority::Urgent > Priority::High && Priority::Low > Priority::None);
        let bars = [
            Priority::None,
            Priority::Low,
            Priority::Medium,
            Priority::High,
        ]
        .map(Priority::bars);
        assert_eq!(bars, [0, 1, 2, 3]);
    }

    #[test]
    fn only_an_open_task_past_its_day_is_overdue() {
        let today = date(2026, 9, 27);
        let late = Task::new("a", "A").due(date(2026, 9, 26));
        assert!(late.overdue(today));
        assert!(!late.clone().status(Status::Canceled).overdue(today));
        assert!(!late.due(today).overdue(today));
    }

    #[test]
    fn an_issue_reads_as_its_team_and_number() {
        assert_eq!(IssueId::new("ENG", 123).to_string(), "ENG-123");
    }

    #[test]
    #[should_panic(expected = "issue team key eng is not one to five capitals")]
    fn a_team_key_is_in_capitals() {
        IssueId::new("eng", 1);
    }
}
