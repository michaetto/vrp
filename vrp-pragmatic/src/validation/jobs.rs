use super::*;
use crate::extensions::MultiDimensionalCapacity;

/// Checks that plan has no jobs with duplicate ids.
fn check_e1000_no_jobs_with_duplicate_ids(ctx: &ValidationContext) -> Result<(), ValidationError> {
    get_duplicates(ctx.jobs().map(|job| &job.id)).map_or(Ok(()), |ids| {
        Err(ValidationError::new(
            "E1000".to_string(),
            format!("duplicated job ids: {}", ids.join(", ")),
            "remove jobs with the same ids".to_string(),
        ))
    })
}

/// Checks that jobs have proper demand.
fn check_e1001_correct_job_types_demand(ctx: &ValidationContext) -> Result<(), ValidationError> {
    let ids = ctx
        .jobs()
        .filter(|job| {
            job.pickups
                .as_ref()
                .iter()
                .chain(job.deliveries.as_ref().iter())
                .chain(job.replacements.as_ref().iter())
                .flat_map(|tasks| tasks.iter())
                .any(|task| task.demand.is_none())
                || job.services.iter().flat_map(|tasks| tasks.iter()).any(|task| task.demand.is_some())
        })
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(
            "E1001".to_string(),
            format!("invalid job task demand in jobs: {}", ids.join(", ")),
            "correct demand based on job task type".to_string(),
        ))
    }
}

/// Checks that sum of pickup/delivery demand should be equal.
fn check_e1002_multiple_pickups_deliveries_demand(ctx: &ValidationContext) -> Result<(), ValidationError> {
    let has_tasks = |tasks: &Option<Vec<JobTask>>| tasks.as_ref().map_or(false, |tasks| tasks.len() > 0);
    let get_demand = |tasks: &Option<Vec<JobTask>>| {
        if let Some(tasks) = tasks {
            tasks
                .iter()
                .map(|task| {
                    task.demand.clone().map_or_else(
                        || MultiDimensionalCapacity::default(),
                        |demand| MultiDimensionalCapacity::new(demand),
                    )
                })
                .sum()
        } else {
            MultiDimensionalCapacity::default()
        }
    };

    let ids = ctx
        .jobs()
        .filter(|job| has_tasks(&job.pickups) && has_tasks(&job.deliveries))
        .filter(|job| get_demand(&job.pickups) - get_demand(&job.deliveries) != MultiDimensionalCapacity::default())
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(
            "E1002".to_string(),
            format!("invalid pickup and delivery demand in jobs: {}", ids.join(", ")),
            "correct demand so that sum of pickups equal to sum of deliveries".to_string(),
        ))
    }
}

/// Checks that job's time windows are correct.
fn check_e1003_time_window_correctness(ctx: &ValidationContext) -> Result<(), ValidationError> {
    let has_invalid_tws = |tasks: &Option<Vec<JobTask>>| {
        tasks.as_ref().map_or(false, |tasks| {
            tasks
                .iter()
                .flat_map(|task| task.places.iter())
                .filter_map(|place| place.times.as_ref())
                .any(|tws| !check_raw_time_windows(tws, false))
        })
    };

    let ids = ctx
        .jobs()
        .filter(|job| has_invalid_tws(&job.pickups) || has_invalid_tws(&job.deliveries))
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(
            "E1003".to_string(),
            format!("invalid time windows in jobs: {}", ids.join(", ")),
            "change job task place time windows so that they don't intersect".to_string(),
        ))
    }
}

/// Validates jobs from the plan.
pub fn validate_jobs(ctx: &ValidationContext) -> Result<(), Vec<ValidationError>> {
    let errors = check_e1000_no_jobs_with_duplicate_ids(ctx)
        .err()
        .iter()
        .cloned()
        .chain(check_e1001_correct_job_types_demand(ctx).err().iter().cloned())
        .chain(check_e1002_multiple_pickups_deliveries_demand(ctx).err().iter().cloned())
        .chain(check_e1003_time_window_correctness(ctx).err().iter().cloned())
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
