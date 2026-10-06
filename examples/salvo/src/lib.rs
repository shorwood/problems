//! salvo handlers and application routes.

use problems::{IntoReport, Report};

use salvo::{Router, handler};

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

#[handler]
fn problem() -> Report<CreateProblem> {
    CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report()
}

/// Builds application routes without binding a listener.
pub fn app() -> Router {
    Router::with_path("problem").get(problem)
}
