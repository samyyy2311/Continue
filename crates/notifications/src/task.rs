// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use protocol::v1::NotificationPost;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCompletionReport {
    pub task_name: String,
    pub duration: Duration,
    pub success: bool,
    pub exit_code: Option<i32>,
}

impl TaskCompletionReport {
    pub fn new(
        task_name: impl Into<String>,
        duration: Duration,
        success: bool,
        exit_code: Option<i32>,
    ) -> Self {
        Self {
            task_name: task_name.into(),
            duration,
            success,
            exit_code,
        }
    }

    pub fn to_notification_post(&self) -> NotificationPost {
        let status_desc = if self.success { "succeeded" } else { "failed" };
        let code_desc = match self.exit_code {
            Some(code) => format!("code {}", code),
            None => "terminated".to_string(),
        };

        let body = format!(
            "{} in {:.1}s ({})",
            status_desc,
            self.duration.as_secs_f32(),
            code_desc
        );

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        NotificationPost {
            notification_id: format!("task-{}", now_ms),
            package_name: "dev.continue.tasks".to_string(),
            app_name: "Task Monitor".to_string(),
            title: format!("Task: {}", self.task_name),
            body,
            timestamp: now_ms,
            actions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_successful_task_notification() {
        let report =
            TaskCompletionReport::new("cargo build", Duration::from_millis(4200), true, Some(0));

        let post = report.to_notification_post();
        assert_eq!(post.title, "Task: cargo build");
        assert!(post.body.contains("succeeded in 4.2s (code 0)"));
        assert_eq!(post.package_name, "dev.continue.tasks");
    }

    #[test]
    fn formats_failed_task_notification() {
        let report =
            TaskCompletionReport::new("test suite", Duration::from_millis(1500), false, Some(1));

        let post = report.to_notification_post();
        assert_eq!(post.title, "Task: test suite");
        assert!(post.body.contains("failed in 1.5s (code 1)"));
    }
}
