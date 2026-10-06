//! poem handlers and application routes.

use problems::{IntoReport, Report};

use poem::{Route, handler};

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
fn problem() -> Result<(), Report<CreateProblem>> {
    create()
}

fn create() -> Result<(), Report<CreateProblem>> {
    Err(CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report())
}

/// Demonstrates conversion through `?` into Poem's error response.
#[handler]
fn question_mark() -> poem::Result<()> {
    create()?;
    Ok(())
}

/// Renders a report directly through Poem's response adapter.
#[handler]
fn direct() -> Report<CreateProblem> {
    CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report()
}

/// Builds application routes without binding a listener.
pub fn app() -> Route {
    Route::new()
        .at("/problem", poem::get(problem))
        .at("/question-mark", poem::get(question_mark))
        .at("/direct", poem::get(direct))
}
