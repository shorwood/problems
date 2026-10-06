//! warp handlers and application routes.

use problems::{IntoReport, Report};

use warp::{Filter, Reply};

/**************************************/
/* Public Problem Contract            */
/**************************************/

#[derive(Debug, thiserror::Error, problems::Problem)]
enum CreateProblem {
    #[error("private duplicate diagnostic: {name}")]
    #[problem(
        type_uri = "urn:example:name-conflict",
        status = 409,
        title = "Name conflict",
        detail = "The name '{name}' is already in use."
    )]
    NameConflict { name: String },
}

/**************************************/
/* Request Handlers and Routes        */
/**************************************/

#[derive(Debug)]
struct AppRejection(Report<CreateProblem>);

impl warp::reject::Reject for AppRejection {}

#[derive(Debug)]
struct Unrelated;
impl warp::reject::Reject for Unrelated {}

async fn rejected_problem() -> Result<&'static str, warp::Rejection> {
    Err(warp::reject::custom(AppRejection(
        CreateProblem::NameConflict {
            name: "example".into(),
        }
        .into_report(),
    )))
}

async fn recover_problem(
    rejection: warp::Rejection,
) -> Result<warp::reply::Response, warp::Rejection> {
    if let Some(problem) = rejection.find::<AppRejection>() {
        return Ok((&problem.0).into_response());
    }
    Err(rejection)
}

fn recovery_app() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("recovered")
        .and(warp::path::end())
        .and(warp::get())
        .and_then(rejected_problem)
        .recover(recover_problem)
}

fn direct_app() -> impl Filter<Extract = (Report<CreateProblem>,), Error = warp::Rejection> + Clone
{
    warp::path("problem")
        .and(warp::path::end())
        .and(warp::get())
        .map(|| {
            CreateProblem::NameConflict {
                name: "example".into(),
            }
            .into_report()
        })
}

/// Builds direct report and application-owned rejection recovery routes.
pub fn app() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    direct_app().or(recovery_app()).or(warp::path("unrelated")
        .and(warp::path::end())
        .and_then(|| async { Err::<&'static str, _>(warp::reject::custom(Unrelated)) })
        .recover(recover_problem))
}
