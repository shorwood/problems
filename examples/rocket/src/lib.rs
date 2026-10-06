//! rocket handlers and application routes.

use problems::{IntoReport, Report};

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

#[rocket::get("/problem")]
fn problem() -> Result<(), Report<CreateProblem>> {
    Err(CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report())
}

/// Builds application routes without binding a listener.
pub fn app() -> rocket::Rocket<rocket::Build> {
    rocket::build().mount("/", rocket::routes![problem])
}
