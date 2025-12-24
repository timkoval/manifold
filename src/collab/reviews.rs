//! Review and approval workflow

use super::{Review, ReviewStatus};
use anyhow::{anyhow, Result};

/// Review manager for approval workflows
pub struct ReviewManager;

impl ReviewManager {
    /// Create a new review request
    pub fn create_review(
        spec_id: String,
        requester: String,
        reviewer: String,
    ) -> Review {
        let now = chrono::Utc::now().timestamp();
        Review {
            id: uuid::Uuid::new_v4().to_string(),
            spec_id,
            requester,
            reviewer,
            status: ReviewStatus::Pending,
            comment: None,
            requested_at: now,
            reviewed_at: None,
        }
    }

    /// Approve a review
    pub fn approve(review: &mut Review, reviewer: &str, comment: Option<String>) -> Result<()> {
        if review.status != ReviewStatus::Pending {
            return Err(anyhow!("Review is not in pending state"));
        }

        if review.reviewer != reviewer {
            return Err(anyhow!("Only the assigned reviewer can approve"));
        }

        review.status = ReviewStatus::Approved;
        review.comment = comment;
        review.reviewed_at = Some(chrono::Utc::now().timestamp());
        Ok(())
    }

    /// Reject a review
    pub fn reject(review: &mut Review, reviewer: &str, comment: String) -> Result<()> {
        if review.status != ReviewStatus::Pending {
            return Err(anyhow!("Review is not in pending state"));
        }

        if review.reviewer != reviewer {
            return Err(anyhow!("Only the assigned reviewer can reject"));
        }

        review.status = ReviewStatus::Rejected;
        review.comment = Some(comment);
        review.reviewed_at = Some(chrono::Utc::now().timestamp());
        Ok(())
    }

    /// Cancel a review request
    /// Part of review workflow API
    #[allow(dead_code)]
    pub fn cancel(review: &mut Review, requester: &str) -> Result<()> {
        if review.status != ReviewStatus::Pending {
            return Err(anyhow!("Review is not in pending state"));
        }

        if review.requester != requester {
            return Err(anyhow!("Only the requester can cancel"));
        }

        review.status = ReviewStatus::Cancelled;
        review.reviewed_at = Some(chrono::Utc::now().timestamp());
        Ok(())
    }

    /// Format review for display
    pub fn format_review(review: &Review) -> String {
        let status_emoji = match review.status {
            ReviewStatus::Pending => "⏳",
            ReviewStatus::Approved => "✅",
            ReviewStatus::Rejected => "❌",
            ReviewStatus::Cancelled => "🚫",
        };

        let mut output = format!(
            "{} Review {} - {}\n",
            status_emoji, review.id, review.status
        );
        output.push_str(&format!("  Spec: {}\n", review.spec_id));
        output.push_str(&format!("  Requester: {}\n", review.requester));
        output.push_str(&format!("  Reviewer: {}\n", review.reviewer));

        let requested = chrono::DateTime::from_timestamp(review.requested_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        output.push_str(&format!("  Requested: {}\n", requested));

        if let Some(reviewed_at) = review.reviewed_at {
            let reviewed = chrono::DateTime::from_timestamp(reviewed_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            output.push_str(&format!("  Reviewed: {}\n", reviewed));
        }

        if let Some(comment) = &review.comment {
            output.push_str(&format!("  Comment: {}\n", comment));
        }

        output
    }

    /// Check if spec has pending reviews
    /// Utility for workflow validation
    #[allow(dead_code)]
    pub fn has_pending_reviews(reviews: &[Review]) -> bool {
        reviews.iter().any(|r| r.status == ReviewStatus::Pending)
    }

    /// Check if spec is approved
    /// Utility for workflow validation
    #[allow(dead_code)]
    pub fn is_approved(reviews: &[Review]) -> bool {
        !reviews.is_empty() && reviews.iter().all(|r| r.status == ReviewStatus::Approved)
    }

    /// Get review statistics
    pub fn get_stats(reviews: &[Review]) -> ReviewStats {
        let mut stats = ReviewStats::default();
        
        for review in reviews {
            match review.status {
                ReviewStatus::Pending => stats.pending += 1,
                ReviewStatus::Approved => stats.approved += 1,
                ReviewStatus::Rejected => stats.rejected += 1,
                ReviewStatus::Cancelled => stats.cancelled += 1,
            }
        }

        stats.total = reviews.len();
        stats
    }
}

/// Review statistics
#[derive(Debug, Default)]
pub struct ReviewStats {
    pub total: usize,
    pub pending: usize,
    pub approved: usize,
    pub rejected: usize,
    pub cancelled: usize,
}

impl ReviewStats {
    pub fn format(&self) -> String {
        format!(
            "Total: {} | ⏳ Pending: {} | ✅ Approved: {} | ❌ Rejected: {} | 🚫 Cancelled: {}",
            self.total, self.pending, self.approved, self.rejected, self.cancelled
        )
    }
}
